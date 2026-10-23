"use strict";

// One initial Framework startup per process. Observe QQ's real initialization;
// never create/start a native session or invent a completion notification.
function createReadyRelay() {
  let generation = 0;
  let readyGeneration = -1;
  let callback = null;
  let delivered = false;
  const flush = () => {
    if (delivered || !callback || generation === 0 || readyGeneration !== generation) return;
    const notify = callback;
    callback = null;
    delivered = true;
    notify();
  };
  return {
    begin() {
      generation += 1;
      readyGeneration = -1;
      return generation;
    },
    observe(epoch, info) {
      if (epoch !== generation) return;
      if (info?.is_init !== true) {
        readyGeneration = -1;
        return;
      }
      readyGeneration = epoch;
      flush();
    },
    register(fn) {
      if (typeof fn !== 'function') throw new TypeError('Expected initialization callback');
      if (callback || delivered) throw new Error('Only one initial Framework readiness callback is supported');
      callback = fn;
      flush();
    },
    getSnapshot() {
      return {
        generation,
        ready: generation > 0 && readyGeneration === generation,
        registered: !!callback,
        delivered,
      };
    },
  };
}

function forwardingView(native, overrides = {}) {
  const methods = new Map();
  return new Proxy({}, {
    get(_target, key) {
      if (Object.prototype.hasOwnProperty.call(overrides, key)) return overrides[key];
      const value = Reflect.get(native, key, native);
      if (typeof value !== 'function') return value;
      if (!methods.has(key) || methods.get(key).source !== value) {
        methods.set(key, { source: value, bound: value.bind(native) });
      }
      return methods.get(key).bound;
    },
    getPrototypeOf() { return Object.getPrototypeOf(native); },
    has(_target, key) { return key in native; },
    ownKeys() { return Reflect.ownKeys(native); },
    getOwnPropertyDescriptor(_target, key) {
      const descriptor = Object.getOwnPropertyDescriptor(native, key);
      return descriptor ? { ...descriptor, configurable: true } : undefined;
    },
  });
}

function createObservedSession(session, relay) {
  const init = session.init;
  if (typeof init !== 'function') throw new Error('Missing native session init');
  return forwardingView(session, {
    init(...args) {
      const epoch = relay.begin();
      const listener = args[3];
      if (!listener || typeof listener.onOpentelemetryInit !== 'function') {
        return Reflect.apply(init, session, args);
      }
      const originalCallback = listener.onOpentelemetryInit;
      args[3] = forwardingView(listener, {
        onOpentelemetryInit(...eventArgs) {
          const result = Reflect.apply(originalCallback, listener, eventArgs);
          relay.observe(epoch, eventArgs[0]);
          return result;
        },
      });
      return Reflect.apply(init, session, args);
    },
  });
}

function installWrapperBridge(processObject, wrapperPath, relay, onSession, fallbackDataPath) {
  const path = require('node:path');
  if (!path.isAbsolute(wrapperPath)) throw new Error('Absolute wrapper path required');
  if (fallbackDataPath !== undefined && !path.isAbsolute(fallbackDataPath)) throw new Error('Absolute fallback data path required');
  const originalDlopen = processObject.dlopen;
  if (typeof originalDlopen !== 'function' || typeof onSession !== 'function') {
    throw new TypeError('Native module loader and session callback required');
  }
  let captured = false;
  function observedDlopen(module, filename, ...rest) {
    const result = Reflect.apply(originalDlopen, this, [module, filename, ...rest]);
    if (typeof filename !== 'string' || path.resolve(filename) !== path.resolve(wrapperPath)) return result;
    const wrapper = module.exports;
    // Native Mac QQ may return no shared data path. Keep nonempty native paths;
    // only supply the host's isolated QQ data directory for the missing value.
    if (fallbackDataPath) {
      const nativeUtil = wrapper.NodeQQNTWrapperUtil;
      const nativeDataPath = nativeUtil?.getNTUserDataInfoConfig;
      if (typeof nativeDataPath !== 'function') throw new Error('Missing native data path accessor');
      function UtilClass(...args) {
        return Reflect.construct(nativeUtil, args, new.target === UtilClass ? nativeUtil : new.target);
      }
      Object.setPrototypeOf(UtilClass, nativeUtil);
      UtilClass.prototype = nativeUtil.prototype;
      Object.defineProperty(UtilClass, 'getNTUserDataInfoConfig', {
        value: function(...args) {
          const value = Reflect.apply(nativeDataPath, nativeUtil, args);
          return value === '' || value === null || value === undefined ? fallbackDataPath : value;
        },
      });
      const utilDescriptor = Object.getOwnPropertyDescriptor(wrapper, 'NodeQQNTWrapperUtil');
      if (!utilDescriptor || !Object.prototype.hasOwnProperty.call(utilDescriptor, 'value') || (!utilDescriptor.configurable && !utilDescriptor.writable)) {
        throw new Error('Native utility class export is not replaceable');
      }
      Object.defineProperty(wrapper, 'NodeQQNTWrapperUtil', { ...utilDescriptor, value: UtilClass });
    }
    if (captured) return result;
    captured = true;
    const nativeClass = wrapper.NodeIQQNTWrapperSession;
    const nativeGet = nativeClass?.getNTWrapperSession;
    if (typeof nativeGet !== 'function') throw new Error('Missing named native session getter');
    const descriptor = Object.getOwnPropertyDescriptor(wrapper, 'NodeIQQNTWrapperSession');
    if (!descriptor || !Object.prototype.hasOwnProperty.call(descriptor, 'value') || (!descriptor.configurable && !descriptor.writable)) throw new Error('Native session class export is not replaceable');
    function SessionClass(...args) {
      return Reflect.construct(nativeClass, args, new.target === SessionClass ? nativeClass : new.target);
    }
    Object.setPrototypeOf(SessionClass, nativeClass);
    SessionClass.prototype = nativeClass.prototype;
    let started = false;
    Object.defineProperty(SessionClass, 'getNTWrapperSession', {
      value: function(...args) {
        const nativeSession = Reflect.apply(nativeGet, nativeClass, args);
        if (args[0] !== 'nt_1') return nativeSession;
        const view = createObservedSession(nativeSession, relay);
        if (!started) {
          started = true;
          // The caller, i.e. original QQ, obtained this object. No session create/start here.
          onSession(nativeSession, wrapper.NodeIKernelLoginService.get());
        }
        return view;
      },
      configurable: true,
      writable: true,
    });
    Object.defineProperty(wrapper, 'NodeIQQNTWrapperSession', { ...descriptor, value: SessionClass });
    return result;
  }
  processObject.dlopen = observedDlopen;
  return () => {
    if (processObject.dlopen === observedDlopen) processObject.dlopen = originalDlopen;
  };
}

module.exports = { createReadyRelay, createObservedSession, installWrapperBridge };
