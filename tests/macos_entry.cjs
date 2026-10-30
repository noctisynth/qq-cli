"use strict";
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { layout, start } = require('../src/napcat/runtime/macos/adapter/entry.cjs');

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
  const paths = new Map(); const events = []; const failures = []; const deferred = [];
  fs.mkdirSync(path.join(root,'config'));
  fs.writeFileSync(path.join(root,'config/onebot11.json'), '{}');
  const entryPath = path.join(root, 'adapter', 'synthetic', 'main.cjs');
  const packageMain = path.relative(path.join(resources, 'app'), entryPath);
  let launcher;
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
    fs, process:processObject, entryPath,
    defer:callback=>deferred.push(callback), getLauncher:()=>launcher,
    app:{isReady:()=>false,setPath(k,v){events.push('path:'+k);paths.set(k,v);},getPath:k=>paths.get(k)},
    onFailure:code=>failures.push(code),
    load(file){
      if (file === path.join(path.dirname(entryPath),'friend_requests.cjs')) return {install(s){assert.equal(s,session);}};
      if (file === path.join(framework,'nativeLoader.cjs')) { events.push('loader');return {initializeNapCat:(...args)=>initializer(...args)}; }
      assert.equal(file,path.join(resources,'app/application.asar/app_launcher/index.js'));
      launcher={installPathPkgJson:{main:packageMain, name:'qq-chat'}};
      events.push('original');const module={};processObject.dlopen(module,path.join(resources,'app/wrapper.node'));
      module.exports.NodeIQQNTWrapperSession.getNTWrapperSession('nt_1').init(null,null,null,{onOpentelemetryInit(){events.push('qq-ready');}});
      return 42;
    },
  };
  return {config:{instanceRoot:root,frameworkRoot:framework,profileHome:path.join(root,'profile-home')},host,events,failures,paths,root,framework,resources,deferred,packageMain,getLauncher:()=>launcher,setInitializer:fn=>{initializer=fn;}};
}

test('official launcher sees its original main after its deferred initialization', t => {
  const f=fixture(t);
  const result=start(f.config,f.host);
  const launcher=f.getLauncher(); const metadata=launcher.installPathPkgJson;
  assert.equal(metadata.main,f.packageMain);
  assert.equal(f.deferred.length,1);
  f.deferred.shift()();
  assert.equal(launcher.installPathPkgJson,metadata);
  assert.deepEqual(metadata,{main:'./application.asar/app_launcher/index.js',name:'qq-chat'});
  assert.equal(result.original,42);
  assert.deepEqual(f.failures,[]);
});

test('launcher lookup runs after the original entry and refuses replaced metadata', t => {
  for (const make of [
    ()=>undefined,
    ()=>({installPathPkgJson:{main:'unexpected-entry'}}),
    f=>({installPathPkgJson:Object.freeze({main:f.packageMain})}),
    ()=>({installPathPkgJson:{get main(){throw Error('private accessor must not run');}}}),
  ]) {
    const f=fixture(t); const invalid=make(f); let calls=0;
    const before=invalid && Object.getOwnPropertyDescriptor(invalid.installPathPkgJson,'main');
    f.host.getLauncher=()=>{calls++; return invalid;};
    start(f.config,f.host);
    assert.equal(calls,0);
    assert.equal(f.deferred.length,1);
    f.deferred.shift()();
    assert.equal(calls,1);
    assert.deepEqual(f.failures,['QQ_LAUNCHER_ENTRY_RESTORE_FAILED']);
    if (invalid) assert.deepEqual(Object.getOwnPropertyDescriptor(invalid.installPathPkgJson,'main'),before);
  }
});

test('deferred launcher metadata preserves descriptors and accepts the original entry', t => {
  const f=fixture(t); let launcher;
  f.host.getLauncher=()=>launcher;
  start(f.config,f.host);
  const metadata={};
  Object.defineProperty(metadata,'main',{value:f.packageMain,writable:true,enumerable:false,configurable:false});
  launcher={installPathPkgJson:metadata};
  f.deferred[0]();
  assert.deepEqual(Object.getOwnPropertyDescriptor(metadata,'main'),{
    value:'./application.asar/app_launcher/index.js',writable:true,enumerable:false,configurable:false,
  });
  Object.freeze(metadata);
  f.deferred[0]();
  assert.deepEqual(f.failures,[]);
});

test('paths precede Framework load; official entry owns session and readiness order', t => {
  const f=fixture(t);const original=f.host.process.dlopen;
  const result=start(f.config,f.host);assert.equal(result.original,42);
  assert.equal(f.events.filter(x=>x.startsWith('path:')).length,11);
  assert.deepEqual(f.events.slice(11),['loader','original','framework','qq-ready','ready']);
  assert.equal(f.failures.length,0);result.restore();assert.equal(f.host.process.dlopen,original);
});

test('inherited data stays in its original home while temporary paths use the new run', t => {
  const f=fixture(t);
  const profile=path.join(path.dirname(f.root),'previous run','profile-home');
  fs.cpSync(path.join(f.root,'profile-home'),profile,{recursive:true});
  fs.writeFileSync(path.join(profile,'synthetic-session'),'preserve');
  f.config.profileHome=profile;
  start(f.config,f.host);
  assert.equal(f.paths.get('home'),path.join(profile,'Library/Containers/com.tencent.qq/Data'));
  assert.equal(f.paths.get('userData'),path.join(profile,'Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ'));
  assert.equal(f.paths.get('temp'),path.join(f.root,'temp'));
  assert.equal(f.paths.get('logs'),path.join(f.root,'logs'));
  assert.equal(fs.readFileSync(path.join(profile,'synthetic-session'),'utf8'),'preserve');
  assert.equal(fs.existsSync(path.join(f.root,'profile-home/synthetic-session')),false);
});

test('profile paths outside the runtime parent or inside Framework are refused before effects', t => {
  for(const choose of [f=>path.join(f.framework,'profile-home'),f=>path.join(path.dirname(f.root),'unrelated/owner/profile-home')]) {
    const f=fixture(t);f.config.profileHome=choose(f);fs.mkdirSync(f.config.profileHome,{recursive:true});
    assert.throws(()=>start(f.config,f.host),/Unexpected profile home/);
    assert.deepEqual(f.events,[]);
  }
});

test('layout requires physical archive stats and keeps rejecting directory substitutes', t => {
  const f=fixture(t);
  const archive=path.join(f.resources,'app/application.asar');
  const virtualFs=new Proxy(fs,{get(target,key){
    if(key==='lstatSync') return file=>fs.lstatSync(file===archive?path.dirname(file):file);
    return target[key];
  }});
  assert.throws(()=>layout(f.config,virtualFs),/Unexpected adapter path kind/);
  assert.equal(layout(f.config,fs).executable,f.host.process.execPath);
  fs.unlinkSync(archive);fs.mkdirSync(archive);
  assert.throws(()=>layout(f.config,fs),/Unexpected adapter path kind/);
});

test('wrong host, late startup and missing directories cause no module loading', t => {
  for (const change of [f=>{f.host.process.arch='x64';},f=>{f.host.process.execPath='/other/QQ';},f=>{f.host.process.resourcesPath='/other';},f=>{f.host.app.isReady=()=>true;},f=>{f.host.entryPath='/other/main.cjs';},f=>fs.rmdirSync(path.join(f.root,'logs'))]) {
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
