"use strict";
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { start } = require('../src/napcat/runtime/macos_adapter/entry.cjs');

function fixture(t) {
  const base = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'qq-entry-')));
  t.after(() => fs.rmSync(base, {recursive:true, force:true}));
  const root = path.join(base, 'instance with spaces');
  const framework = path.join(base, 'framework');
  const resources = path.join(root, 'QQ.app/Contents/Resources');
  const executable = path.join(root, 'QQ.app/Contents/MacOS/QQ');
  const home = path.join(root, 'profile-home/Library/Containers/com.tencent.qq/Data');
  for (const dir of ['Desktop','Documents','Downloads','Music','Pictures','Movies','Library/Application Support/QQ']) {
    fs.mkdirSync(path.join(home, dir), {recursive:true});
  }
  for (const dir of ['temp','logs']) fs.mkdirSync(path.join(root, dir));
  for (const file of [executable, path.join(resources,'app/wrapper.node'), path.join(resources,'app/application.asar'), path.join(framework,'nativeLoader.cjs')]) {
    fs.mkdirSync(path.dirname(file), {recursive:true}); fs.writeFileSync(file, 'synthetic inert file');
  }
  const paths = new Map(); const events = []; const failures = [];
  const session = {init(a,b,c,listener){listener.onOpentelemetryInit({is_init:true});}};
  function Session() {} Session.getNTWrapperSession = () => session;
  function Util() {} Util.getNTUserDataInfoConfig = () => '';
  const login = {};
  const wrapper = {NodeIQQNTWrapperSession:Session, NodeQQNTWrapperUtil:Util, NodeIKernelLoginService:{get:()=>login}};
  const processObject = {platform:'darwin',arch:'arm64',execPath:executable,resourcesPath:resources,dlopen(module){module.exports=wrapper;}};
  let initializer = (s,l,register) => {
    assert.equal(s,session); assert.equal(l,login);
    events.push('framework'); register(()=>events.push('ready'));
  };
  const host = {
    fs, process:processObject,
    app:{isReady:()=>false,setPath(k,v){events.push('path:'+k);paths.set(k,v);},getPath:k=>paths.get(k)},
    onFailure:code=>failures.push(code),
    load(file){
      if (file === path.join(framework,'nativeLoader.cjs')) { events.push('loader');return {initializeNapCat:(...args)=>initializer(...args)}; }
      assert.equal(file,path.join(resources,'app/application.asar/app_launcher/index.js'));
      events.push('original');const module={};processObject.dlopen(module,path.join(resources,'app/wrapper.node'));
      module.exports.NodeIQQNTWrapperSession.getNTWrapperSession('nt_1').init(null,null,null,{onOpentelemetryInit(){events.push('qq-ready');}});
      return 42;
    },
  };
  return {config:{instanceRoot:root,frameworkRoot:framework},host,events,failures,paths,root,framework,resources,setInitializer:fn=>{initializer=fn;}};
}

test('paths precede Framework load; official entry owns session and readiness order', t => {
  const f=fixture(t);const original=f.host.process.dlopen;
  const result=start(f.config,f.host);assert.equal(result.original,42);
  assert.equal(f.events.filter(x=>x.startsWith('path:')).length,11);
  assert.deepEqual(f.events.slice(11),['loader','original','framework','qq-ready','ready']);
  assert.equal(f.failures.length,0);result.restore();assert.equal(f.host.process.dlopen,original);
});

test('wrong host, late startup and missing directories cause no module loading', t => {
  for (const change of [f=>{f.host.process.arch='x64';},f=>{f.host.process.execPath='/other/QQ';},f=>{f.host.process.resourcesPath='/other';},f=>{f.host.app.isReady=()=>true;},f=>fs.rmdirSync(path.join(f.root,'logs'))]) {
    const f=fixture(t);change(f);assert.throws(()=>start(f.config,f.host));assert.deepEqual(f.events,[]);
  }
});

test('aliases, hard links, overlapping roots and traversal are rejected before effects', t => {
  for (const change of [
    f=>{fs.renameSync(path.join(f.root,'logs'),path.join(f.root,'real-logs'));fs.symlinkSync('real-logs',path.join(f.root,'logs'));},
    f=>fs.linkSync(path.join(f.framework,'nativeLoader.cjs'),path.join(f.framework,'alias')),
    f=>{f.config.frameworkRoot=f.root;},
    f=>{f.config.instanceRoot += '/../instance with spaces';},
  ]) { const f=fixture(t);change(f);assert.throws(()=>start(f.config,f.host));assert.deepEqual(f.events,[]); }
});

test('a rejected Electron path prevents upstream loading', t => {
  const f=fixture(t);f.host.app.getPath=()=>'/unexpected';
  assert.throws(()=>start(f.config,f.host));assert.ok(!f.events.includes('loader'));
});

test('original entry exception is preserved and observer restored', t => {
  const f=fixture(t);const original=f.host.process.dlopen;const load=f.host.load;const error=new Error('synthetic original failure');
  f.host.load=file=>file.endsWith('nativeLoader.cjs')?load(file):(()=>{throw error;})();
  assert.throws(()=>start(f.config,f.host),e=>e===error);assert.equal(f.host.process.dlopen,original);
});

test('async Framework failure emits only a fixed private-safe code', async t => {
  const f=fixture(t);f.setInitializer(()=>Promise.reject(new Error('synthetic private error')));
  start(f.config,f.host);await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual(f.failures,['FRAMEWORK_INITIALIZATION_FAILED']);assert.ok(!f.events.includes('ready'));
});

test('synchronous Framework failure is preserved and observer restored', t => {
  const f=fixture(t);const original=f.host.process.dlopen;const error=new Error('synthetic failure');
  f.setInitializer(()=>{throw error;});assert.throws(()=>start(f.config,f.host),e=>e===error);
  assert.equal(f.host.process.dlopen,original);assert.deepEqual(f.failures,['FRAMEWORK_INITIALIZATION_FAILED']);
});

test('late readiness registration cannot release Framework after initialization failure', async t => {
  const f=fixture(t);let register;let released=false;
  f.setInitializer((session,login,callback)=>{register=callback;return Promise.reject(new Error('synthetic failure'));});
  start(f.config,f.host);await new Promise(resolve=>setImmediate(resolve));
  register(()=>{released=true;});assert.equal(released,false);
  assert.deepEqual(f.failures,['FRAMEWORK_INITIALIZATION_FAILED']);
});
