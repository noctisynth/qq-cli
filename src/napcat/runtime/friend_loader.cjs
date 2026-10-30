"use strict";
const path = require('node:path');
const fs = require('node:fs');
// The launcher supplies the already authenticated installation as one literal
// environment value; no shell, alternate session, or discovery fallback.
async function initializeNapCat(session, login, register) {
  try {
    const root = process.env.QQ_CLI_FRAMEWORK_ROOT;
    if (!root || !path.isAbsolute(root)) throw new Error('FRAMEWORK_ROOT');
    const { NCoreInitFramework } = await import(require('node:url').pathToFileURL(path.join(root, 'napcat.mjs')).href);
    await NCoreInitFramework(session, login, register);
    const config = JSON.parse(fs.readFileSync(path.join(__dirname, 'onebot11.json'), 'utf8'));
    require('./friend_requests.cjs').install(session, config);
  } catch { process.stderr.write('QQ_CLI_FRIEND_ADAPTER_FAILED\n'); }
}
module.exports = { initializeNapCat };
