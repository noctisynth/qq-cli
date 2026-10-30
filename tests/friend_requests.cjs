"use strict";
const test = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const { reader, install } = require('../src/napcat/runtime/friend_requests.cjs');
const request = (overrides = {}) => ({ isInitiator:false, isDecide:false, isDoubt:false, reqType:1, reqTime:'100', friendUid:'u_synthetic', friendNick:'Synthetic', extWords:'synthetic message', ...overrides });
function service() {
  let listener;
  return { calls:0, addKernelBuddyListener(value){listener=value;}, getBuddyReq(){this.calls++;}, emit(rows){listener.onBuddyReqChange({buddyReqs:rows});} };
}
test('pending incoming requests include read requests and exact approval flags', async () => {
  const backend=service(); const read=reader(backend);
  const result=read(); assert.equal(read(),result); assert.equal(backend.calls,1);
  backend.emit([
    request({isUnread:false}), request({reqTime:'101',isInitiator:true}),
    request({reqTime:'102',isDecide:true}), request({reqTime:'103',isDoubt:true}),
    request({reqTime:'104',reqType:13}), request({reqTime:'105',isInitiator:undefined}),
  ]);
  assert.deepEqual(await result,{requests:[{flag:'100',uid:'u_synthetic',nickname:'Synthetic',message:'synthetic message'}],complete:false});
  const empty=read(); backend.emit([]); assert.deepEqual((await empty).requests,[]);
});
test('malformed and ambiguous approval flags fail closed', async () => {
  for (const rows of [[request({reqTime:100})],[request(),request({isDecide:true})],[request({friendUid:''})]]) {
    const backend=service(); const read=reader(backend); const result=read();
    backend.emit(rows); await assert.rejects(result,/REQUEST_PROTOCOL/);
  }
});
test('native timeout never reuses a late response or dispatches another query', async () => {
  const backend=service(); const read=reader(backend,5);
  await assert.rejects(read(),/REQUEST_TIMEOUT/);
  backend.emit([request()]); await assert.rejects(read(),/REQUEST_CHANNEL_UNKNOWN/);
  assert.equal(backend.calls,1);
});
test('synchronous and asynchronous native failures do not leak their details', async () => {
  for (const failure of [()=>{throw new Error('private');},()=>Promise.reject(new Error('private'))]) {
    const backend=service(); backend.getBuddyReq=failure;
    const read=reader(backend);
    await assert.rejects(read(),/REQUEST_CHANNEL_UNKNOWN/);
    await assert.rejects(read(),/REQUEST_CHANNEL_UNKNOWN/);
  }
});
test('Windows loader initializes the same session before installing the original adapter', async t => {
  const fs=require('node:fs'), path=require('node:path'), os=require('node:os');
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'qq-friend-loader-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  const config=path.join(root,'config'), framework=path.join(root,'framework');
  fs.mkdirSync(config);fs.mkdirSync(framework);
  for (const name of ['friend_loader.cjs','friend_requests.cjs']) {
    fs.copyFileSync(path.join(__dirname,'../src/napcat/runtime',name),path.join(config,name));
  }
  fs.writeFileSync(path.join(framework,'napcat.mjs'), 'export async function NCoreInitFramework(session,login,register){if(login!==session.login)throw Error("wrong session");session.ready=true;register(()=>{});}');
  fs.writeFileSync(path.join(config,'onebot11.json'),JSON.stringify({network:{httpServers:[{name:'qq-cli-http',host:'127.0.0.1',enable:true,port:19001,token:'a'.repeat(64)}]}}));
  const backend=service(); const login={}; let registered=0;
  const session={login,ready:false,getBuddyService(){assert.equal(this.ready,true);return backend;}};
  const previous=process.env.QQ_CLI_FRAMEWORK_ROOT;
  const emit=http.Server.prototype.emit;
  t.after(()=>{http.Server.prototype.emit=emit;if(previous===undefined)delete process.env.QQ_CLI_FRAMEWORK_ROOT;else process.env.QQ_CLI_FRAMEWORK_ROOT=previous;});
  process.env.QQ_CLI_FRAMEWORK_ROOT=framework;
  await require(path.join(config,'friend_loader.cjs')).initializeNapCat(session,login,()=>registered++);
  assert.equal(registered,1);assert.equal(session.ready,true);
  assert.notEqual(http.Server.prototype.emit,emit);assert.equal(backend.calls,0);
});
test('managed HTTP route requires the exact bearer and port, bounds input, preserves other routes', async t => {
  const backend=service(); backend.getBuddyReq=function(){this.calls++;queueMicrotask(()=>this.emit([request()]));};
  const server=http.createServer((req,res)=>{req.resume();res.end('original');});
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  t.after(()=>new Promise(resolve=>server.close(resolve)));
  const port=server.address().port, token='a'.repeat(64);
  const restore=install({getBuddyService:()=>backend},{network:{httpServers:[{name:'qq-cli-http',enable:true,host:'127.0.0.1',port,token}]}});
  t.after(restore);
  server.once('synthetic-empty-event',(...args)=>assert.equal(args.length,0));
  server.emit('synthetic-empty-event');
  async function call(path='/qq_cli_get_friend_requests', authorization=`Bearer ${token}`, body='{}', method='POST') {
    return new Promise((resolve,reject)=>{
      const req=http.request({host:'127.0.0.1',port,path,method,agent:false,headers:{Authorization:authorization,'Content-Type':'application/json','Content-Length':Buffer.byteLength(body)}},res=>{
        let bytes='';res.on('data',chunk=>bytes+=chunk);res.on('end',()=>resolve({code:res.statusCode,body:bytes}));
      });req.on('error',reject);req.end(body);
    });
  }
  assert.equal((await call('/get_status')).body,'original');
  assert.equal((await call(undefined,'Bearer wrong')).code,403);
  assert.equal((await call(undefined,undefined,'{}','GET')).code,403);
  assert.equal((await call(undefined,undefined,'{"unexpected":1}')).code,400);
  assert.equal((await call(undefined,undefined,'true')).code,400);
  assert.equal((await call(undefined,undefined,' '.repeat(1025))).code,400);
  assert.equal(backend.calls,0);
  const response=await call(); assert.equal(response.code,200);
  assert.equal(JSON.parse(response.body).data.requests[0].flag,'100');
  assert.equal(backend.calls,1);
  restore(); assert.equal((await call()).body,'original');
});
