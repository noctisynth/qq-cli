"use strict";

// Original, fixed-ABI adapter for the existing managed QQ session. No second
// listener socket, plugin whitelist changes, or upstream-file modifications.
const ROUTE = '/qq_cli_get_friend_requests';
const { timingSafeEqual } = require('node:crypto');

function reader(service, timeout = 10000) {
  let pending;
  let poisoned = false;
  const listener = new Proxy({}, { get(_target, key) {
    if (key !== 'onBuddyReqChange') return () => {};
    return value => {
      if (!pending) return;
      const current = pending;
      pending = undefined;
      clearTimeout(current.timer);
      if (!Array.isArray(value?.buddyReqs) || value.buddyReqs.length > 10000) {
        current.reject(new Error('REQUEST_PROTOCOL'));
        return;
      }
      try {
        const requests = value.buddyReqs.filter(row =>
          row.isInitiator === false && row.isDecide === false &&
          row.reqType === 1 && row.isDoubt === false
        ).map(row => {
          if (typeof row.reqTime !== 'string' || !/^[0-9]{1,32}$/.test(row.reqTime) ||
              typeof row.friendUid !== 'string' || !row.friendUid ||
              typeof row.friendNick !== 'string' || typeof row.extWords !== 'string') {
            throw new Error('REQUEST_PROTOCOL');
          }
          return { flag: row.reqTime, uid: row.friendUid, nickname: row.friendNick, message: row.extWords };
        });
        // reqTime is also the upstream approval lookup key. Ambiguous flags must
        // never be offered as actionable, even if only one match is pending.
        const counts = new Map();
        for (const row of value.buddyReqs) counts.set(row.reqTime, (counts.get(row.reqTime) || 0) + 1);
        if (requests.some(row => counts.get(row.flag) !== 1)) throw new Error('REQUEST_PROTOCOL');
        current.resolve({ requests, complete: false });
      } catch { current.reject(new Error('REQUEST_PROTOCOL')); }
    };
  }});
  service.addKernelBuddyListener(listener);
  return () => {
    if (poisoned) return Promise.reject(new Error('REQUEST_CHANNEL_UNKNOWN'));
    if (pending) return pending.promise;
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    const current = { promise, resolve, reject };
    pending = current;
    current.timer = setTimeout(() => {
      if (pending !== current) return;
      pending = undefined;
      poisoned = true; // Native callbacks have no correlation ID; never misattribute a late response.
      reject(new Error('REQUEST_TIMEOUT'));
    }, timeout);
    try {
      Promise.resolve(service.getBuddyReq()).catch(() => {
        if (pending !== current) return;
        clearTimeout(current.timer);
        pending = undefined;
        poisoned = true;
        reject(new Error('REQUEST_CHANNEL_UNKNOWN'));
      });
    } catch {
      clearTimeout(current.timer); pending = undefined; poisoned = true;
      reject(new Error('REQUEST_CHANNEL_UNKNOWN'));
    }
    return promise;
  };
}

function install(session, config, http = require('node:http')) {
  const server = config?.network?.httpServers?.find(row => row.name === 'qq-cli-http');
  if (!server || server.host !== '127.0.0.1' || server.enable !== true ||
      !Number.isInteger(server.port) || server.port < 1024 || server.port > 65535 ||
      typeof server.token !== 'string' || !/^[a-f0-9]{64}$/.test(server.token)) {
    throw new Error('REQUEST_ADAPTER_CONFIG');
  }
  const read = reader(session.getBuddyService());
  const original = http.Server.prototype.emit;
  const secret = Buffer.from(`Bearer ${server.token}`);
  function emit(event, req, res, ...rest) {
    if (event !== 'request' || req.url !== ROUTE || this.address()?.port !== server.port) {
      return Reflect.apply(original, this, arguments);
    }
    const reply = (code, status, retcode, data) => {
      if (res.destroyed || res.writableEnded) return;
      const bytes = JSON.stringify({ status, retcode, data });
      res.writeHead(code, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(bytes), 'Cache-Control': 'no-store' });
      res.end(bytes);
    };
    const auth = Buffer.from(typeof req.headers.authorization === 'string' ? req.headers.authorization : '');
    if (req.method !== 'POST' || auth.length !== secret.length || !timingSafeEqual(auth, secret)) {
      req.resume(); reply(403, 'failed', 1403, null); return true;
    }
    let size = 0, chunks = [];
    const inputTimer = setTimeout(() => req.destroy(), 10000);
    req.once('end', () => clearTimeout(inputTimer));
    req.once('close', () => clearTimeout(inputTimer));
    req.setTimeout(10000, () => req.destroy());
    req.on('error', () => {});
    req.on('data', chunk => {
      size += chunk.length;
      if (size <= 1024) chunks.push(chunk);
    });
    req.on('end', async () => {
      try {
        if (size > 1024) throw new Error('INVALID');
        const body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
        if (!body || typeof body !== 'object' || Array.isArray(body) || Object.keys(body).length !== 0) throw new Error('INVALID');
      } catch { reply(400, 'failed', 1400, null); return; }
      try {
        const data = await read();
        if (Buffer.byteLength(JSON.stringify(data)) > 1024 * 1024) throw new Error('LIMIT');
        reply(200, 'ok', 0, data);
      } catch { reply(200, 'failed', 1500, null); }
    });
    return true;
  }
  http.Server.prototype.emit = emit;
  return () => { if (http.Server.prototype.emit === emit) http.Server.prototype.emit = original; };
}

module.exports = { reader, install };
