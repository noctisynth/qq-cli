"use strict";
const path = require('node:path');
const { createReadyRelay, installWrapperBridge } = require('./bridge.cjs');
const ORIGINAL_MAIN = './application.asar/app_launcher/index.js';

// The lifecycle supplies authenticated, prepared paths. These checks are a
// second shape check, not a substitute for signing, sandboxing or ownership.
function layout(config, fs) {
  function existing(value, directory) {
    if (typeof value !== 'string' || !path.isAbsolute(value) || path.resolve(value) !== value || /[\x00-\x1f\x7f]/.test(value)) {
      throw new Error('Invalid adapter path');
    }
    if (fs.realpathSync(value) !== value) throw new Error('Adapter path contains aliases');
    const stat = fs.lstatSync(value);
    if (directory ? !stat.isDirectory() : !stat.isFile() || stat.nlink !== 1) {
      throw new Error('Unexpected adapter path kind');
    }
    return value;
  }
  const root = existing(config.instanceRoot, true);
  const framework = existing(config.frameworkRoot, true);
  const profileHome = existing(config.profileHome, true);
  const inside = (parent, child) => child === parent || child.startsWith(parent + path.sep);
  if (root === '/' || framework === '/' || inside(root, framework) || inside(framework, root)) {
    throw new Error('Instance and Framework roots must be separate');
  }
  if (path.basename(profileHome) !== 'profile-home' || path.dirname(path.dirname(profileHome)) !== path.dirname(root) ||
      inside(framework, profileHome) || inside(profileHome, framework)) {
    throw new Error('Unexpected profile home');
  }
  const appRoot = existing(path.join(root, 'QQ.app'), true);
  const resources = existing(path.join(appRoot, 'Contents', 'Resources'), true);
  const qqResources = existing(path.join(resources, 'app'), true);
  const executable = existing(path.join(appRoot, 'Contents', 'MacOS', 'QQ'), false);
  const wrapper = existing(path.join(qqResources, 'wrapper.node'), false);
  // Electron resolves the original entry inside this archive after validation.
  existing(path.join(qqResources, 'application.asar'), false);
  const loader = existing(path.join(framework, 'nativeLoader.cjs'), false);
  const home = path.join(profileHome, 'Library', 'Containers', 'com.tencent.qq', 'Data');
  const paths = {
    home, desktop: path.join(home, 'Desktop'), documents: path.join(home, 'Documents'),
    downloads: path.join(home, 'Downloads'), music: path.join(home, 'Music'),
    pictures: path.join(home, 'Pictures'), videos: path.join(home, 'Movies'),
    appData: path.join(home, 'Library', 'Application Support'),
    userData: path.join(home, 'Library', 'Application Support', 'QQ'),
    temp: path.join(root, 'temp'), logs: path.join(root, 'logs'),
  };
  for (const value of Object.values(paths)) existing(value, true);
  return { resources, qqResources, executable, wrapper, loader, paths };
}

function start(config, host) {
  const { fs, app, process: processObject, load, onFailure, defer, getLauncher, entryPath } = host;
  if ([load, onFailure, defer, getLauncher].some(fn => typeof fn !== 'function')) throw new TypeError('Missing entry host');
  const plan = layout(config, fs);
  if (typeof entryPath !== 'string' || !path.isAbsolute(entryPath) || path.resolve(entryPath) !== entryPath ||
      path.dirname(path.dirname(entryPath)) !== path.join(config.instanceRoot, 'adapter') || path.basename(entryPath) !== 'main.cjs') {
    throw new Error('Unexpected adapter entry');
  }
  const packageMain = path.relative(plan.qqResources, entryPath);
  if (processObject.platform !== 'darwin' || processObject.arch !== 'arm64' ||
      processObject.execPath !== plan.executable || processObject.resourcesPath !== plan.resources || app.isReady()) {
    throw new Error('Adapter must run early in the dedicated Apple Silicon QQ copy');
  }
  for (const [key, value] of Object.entries(plan.paths)) app.setPath(key, value);
  // Do not load nativeLoader until every Electron path is installed and checked.
  for (const [key, value] of Object.entries(plan.paths)) {
    if (app.getPath(key) !== value) throw new Error('Electron rejected an isolated path');
  }
  const { initializeNapCat } = load(plan.loader);
  if (typeof initializeNapCat !== 'function') throw new Error('Missing Framework initializer');
  const relay = createReadyRelay();
  let failed = false;
  const fail = () => {
    if (!failed) {
      failed = true;
      // Never expose upstream exception text, which may contain private data.
      onFailure('FRAMEWORK_INITIALIZATION_FAILED');
    }
  };
  const restore = installWrapperBridge(processObject, plan.wrapper, relay, (session, login) => {
    try {
      Promise.resolve(initializeNapCat(session, login, callback => {
        if (typeof callback !== 'function') throw new TypeError('Expected Framework readiness callback');
        relay.register(() => { if (!failed) callback(); });
      })).catch(fail);
    } catch (error) {
      fail();
      throw error;
    }
  }, plan.paths.userData);
  try {
    const original = load(path.join(plan.qqResources, ORIGINAL_MAIN));
    // QQ initializes this metadata after its entry is required. Restore only
    // its in-memory main, after that initialization; the signed package stays
    // pointed at this adapter for the next process launch.
    defer(() => {
      try {
        const metadata = getLauncher()?.installPathPkgJson;
        if (!metadata || typeof metadata !== 'object') throw new Error('Missing launcher metadata');
        const descriptor = Object.getOwnPropertyDescriptor(metadata, 'main');
        if (!descriptor || !Object.prototype.hasOwnProperty.call(descriptor, 'value')) throw new Error('Unexpected launcher main');
        if (descriptor.value === ORIGINAL_MAIN) return;
        if (descriptor.value !== packageMain || descriptor.writable !== true) throw new Error('Launcher main changed');
        Object.defineProperty(metadata, 'main', { ...descriptor, value: ORIGINAL_MAIN });
      } catch {
        onFailure('QQ_LAUNCHER_ENTRY_RESTORE_FAILED');
      }
    });
    return { original, restore };
  } catch (error) {
    restore();
    throw error;
  }
}

module.exports = { layout, start };
