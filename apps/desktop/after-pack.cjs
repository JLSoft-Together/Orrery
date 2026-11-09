const fs = require('node:fs')
const path = require('node:path')

const WEBGPU_ONLY = ['dxcompiler.dll', 'dxil.dll']

exports.default = async function afterPack(context) {
  if (context.electronPlatformName !== 'win32' || process.env.CCM_KEEP_DXC === '1') return
  for (const name of WEBGPU_ONLY) fs.rmSync(path.join(context.appOutDir, name), { force: true })
}
