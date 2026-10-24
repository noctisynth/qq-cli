"use strict";
const test = require('node:test');
const assert = require('node:assert/strict');
const {createReadyRelay, createObservedSession, installWrapperBridge} = require('../src/napcat/runtime/macos/adapter/bridge.cjs');

test('never releases before a real successful notification; handles early registration', () => {
  const r=createReadyRelay(); let calls=0;r.register(()=>calls++);
  assert.equal(calls,0);const epoch=r.begin();r.observe(epoch,{is_init:false});assert.equal(calls,0);
  r.observe(epoch,{is_init:true});r.observe(epoch,{is_init:true});assert.equal(calls,1);
});
test('handles registration after real initialization, once only', () => {
  const r=createReadyRelay();const epoch=r.begin();r.observe(epoch,{is_init:true});let calls=0;
  r.register(()=>calls++);assert.equal(calls,1);assert.throws(()=>r.register(()=>{}));
});
test('ignores stale generations and truthy non-boolean success', () => {
  const r=createReadyRelay();let calls=0;r.register(()=>calls++);const old=r.begin();const current=r.begin();
  r.observe(old,{is_init:true});r.observe(current,{is_init:1});assert.equal(calls,0);
  r.observe(current,{is_init:true});assert.equal(calls,1);
});

test('read-only native methods and frozen listener are forwarded without mutation', () => {
  const received=[];const native={id:1};
  Object.defineProperty(native,'init',{value:function(...args){assert.equal(this,native);received.push(args);return 17;}});
  Object.defineProperty(native,'service',{value:function(){assert.equal(this,native);return 'service';}});
  const relay=createReadyRelay();let ready=0;relay.register(()=>ready++);
  const view=createObservedSession(native,relay);let originalCalls=0;
  const listener=Object.freeze({onOpentelemetryInit(info){assert.equal(this,listener);originalCalls++;return 23;}, other(){assert.equal(this,listener);return 9;}});
  const args=[{config:true},{depends:true},{dispatcher:true},listener];
  assert.equal(view.init(...args),17);args.slice(0,3).forEach((arg,i)=>assert.equal(received[0][i],arg));
  const passed=received[0][3];assert.equal(passed.other(),9);assert.equal(passed.onOpentelemetryInit({is_init:true}),23);
  assert.equal(ready,1);assert.equal(originalCalls,1);assert.equal(view.service(),'service');
  assert.equal(Object.getOwnPropertyDescriptor(native,'init').writable,false);
});
test('preserves callback exception without falsely signaling readiness', () => {
  let passed;const native={init(...args){passed=args[3];}};const relay=createReadyRelay();let ready=0;relay.register(()=>ready++);
  const view=createObservedSession(native,relay);const error=new Error('original callback failure');
  view.init(null,null,null,{onOpentelemetryInit(){throw error;}});
  assert.throws(()=>passed.onOpentelemetryInit({is_init:true}),e=>e===error);assert.equal(ready,0);
});
test('new unobservable initialization invalidates earlier readiness', () => {
  const relay=createReadyRelay();const epoch=relay.begin();relay.observe(epoch,{is_init:true});
  const native={init(){return 'unchanged';}};const view=createObservedSession(native,relay);
  assert.equal(view.init(null,null,null,{}),'unchanged');let ready=0;relay.register(()=>ready++);assert.equal(ready,0);
});
test('module bridge uses only the official getter and hands the same native object to Framework', () => {
  const created=[];let getterCalls=0;const login={};
  const native={init(){}, getSessionId(){return 'nt_1';}};
  function NativeClass() { throw new Error('Must never construct a session'); }
  Object.defineProperty(NativeClass,'getNTWrapperSession',{value:function(name){assert.equal(this,NativeClass);getterCalls++;return native;}});
  const originalGetter=NativeClass.getNTWrapperSession;
  const wrapper={NodeIQQNTWrapperSession:NativeClass,NodeIKernelLoginService:{get(){return login;}}};
  const fakeProcess={dlopen(module){module.exports=wrapper;return 42;}};const original=fakeProcess.dlopen;
  const relay=createReadyRelay();installWrapperBridge(fakeProcess,'/test/wrapper.node',relay,(...args)=>created.push(args));
  const module={};assert.equal(fakeProcess.dlopen(module,'/test/wrapper.node'),42);assert.notEqual(fakeProcess.dlopen,original);
  assert.equal(module.exports.NodeIQQNTWrapperSession.getNTWrapperSession('other'),native);assert.equal(created.length,0);
  const view=module.exports.NodeIQQNTWrapperSession.getNTWrapperSession('nt_1');assert.notEqual(view,native);
  assert.equal(created.length,1);assert.equal(created[0][0],native);assert.equal(created[0][1],login);
  module.exports.NodeIQQNTWrapperSession.getNTWrapperSession('nt_1');assert.equal(created.length,1);assert.equal(getterCalls,3);
  assert.equal(NativeClass.getNTWrapperSession,originalGetter);
});

test('maps only missing native data paths on every wrapper registration and preserves other modules', () => {
 function NativeSession(){};Object.defineProperty(NativeSession,'getNTWrapperSession',{value(){return {init(){}};}});
 function NativeUtil(){};let nativeValue='';Object.defineProperty(NativeUtil,'getNTUserDataInfoConfig',{value(){assert.equal(this,NativeUtil);return nativeValue;}});
 const fresh=()=>({NodeIQQNTWrapperSession:NativeSession,NodeQQNTWrapperUtil:NativeUtil,NodeIKernelLoginService:{get(){return {};}}});
 const processObject={dlopen(module){module.exports=fresh();return 9;}};const original=processObject.dlopen;
 const undo=installWrapperBridge(processObject,'/qq/wrapper.node',createReadyRelay(),()=>{},'/isolated/QQ');
 for(let i=0;i<2;i++) { const module={};processObject.dlopen(module,'/qq/wrapper.node');assert.equal(module.exports.NodeQQNTWrapperUtil.getNTUserDataInfoConfig(),'/isolated/QQ'); }
 nativeValue='/native/QQ';const third={};processObject.dlopen(third,'/qq/wrapper.node');assert.equal(third.exports.NodeQQNTWrapperUtil.getNTUserDataInfoConfig(),'/native/QQ');
 nativeValue='';const other={};processObject.dlopen(other,'/qq/other.node');assert.equal(other.exports.NodeQQNTWrapperUtil.getNTUserDataInfoConfig(),'');
 undo();assert.equal(processObject.dlopen,original);
});


test('does not inspect private identity or path configuration', () => {
 const config = {};
 for (const key of ['selfUin','desktopPathConfig','defaultFileDownloadPath','d2Key']) {
  Object.defineProperty(config,key,{get(){throw new Error('Private configuration was inspected');}});
 }
 const native={init(...args){assert.equal(args[0],config);return 7;}};
 const view=createObservedSession(native,createReadyRelay());
 assert.equal(view.init(config,null,null,{}),7);
});

test('rejects relative paths before installing a loader observer', () => {
 const processObject={dlopen(){}}; const original=processObject.dlopen;
 assert.throws(()=>installWrapperBridge(processObject,'wrapper.node',createReadyRelay(),()=>{}));
 assert.throws(()=>installWrapperBridge(processObject,'/qq/wrapper.node',createReadyRelay(),()=>{},'relative'));
 assert.equal(processObject.dlopen,original);
});

test('preserves loader receiver, arguments, return and original exceptions', () => {
 const module={}; const failure=new Error('synthetic native failure');
 const processObject={dlopen(...args){assert.equal(this,processObject);assert.deepEqual(args,[module,'/qq/unrelated.node',42]);return 9;}};
 const restore=installWrapperBridge(processObject,'/qq/wrapper.node',createReadyRelay(),()=>assert.fail('unrelated module'));
 assert.equal(processObject.dlopen(module,'/qq/unrelated.node',42),9);
 restore();
 processObject.dlopen=()=>{throw failure;};
 installWrapperBridge(processObject,'/qq/wrapper.node',createReadyRelay(),()=>assert.fail('failed load'));
 assert.throws(()=>processObject.dlopen(module,'/qq/wrapper.node'),error=>error===failure);
});

test('observer restoration never overwrites a later loader owner', () => {
 const processObject={dlopen(){}};
 const restore=installWrapperBridge(processObject,'/qq/wrapper.node',createReadyRelay(),()=>{});
 const replacement=()=>{}; processObject.dlopen=replacement;
 restore(); assert.equal(processObject.dlopen,replacement);
});

test('rejects frozen class exports without changing native methods', () => {
 function NativeClass(){}
 const getter=()=>({init(){}});
 Object.defineProperty(NativeClass,'getNTWrapperSession',{value:getter});
 const wrapper=Object.freeze({NodeIQQNTWrapperSession:NativeClass});
 const processObject={dlopen(module){module.exports=wrapper;}};
 installWrapperBridge(processObject,'/qq/wrapper.node',createReadyRelay(),()=>assert.fail('unsupported export'));
 assert.throws(()=>processObject.dlopen({},'/qq/wrapper.node'));
 assert.equal(NativeClass.getNTWrapperSession,getter);
});

test('preserves native init failure and does not synthesize readiness', () => {
 const failure=new Error('synthetic init failure');
 const relay=createReadyRelay(); let calls=0; relay.register(()=>calls++);
 const view=createObservedSession({init(){throw failure;}},relay);
 assert.throws(()=>view.init(null,null,null,{onOpentelemetryInit(){}}),error=>error===failure);
 assert.equal(calls,0);
});
