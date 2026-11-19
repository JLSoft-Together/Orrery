const UI_GENERATIONS = ['classic', 'next']

function uiGenScript(gen = 'next', opts = {}) {
  if (!UI_GENERATIONS.includes(gen)) throw new Error(`unknown ui generation ${gen}`)
  const sets = [`localStorage.setItem('ccm.uiGeneration', ${JSON.stringify(gen)})`, `localStorage.setItem('ccm.uiMigration', '4.1-next')`]
  if (opts.palette) sets.push(`localStorage.setItem('ccm.nextTheme', ${JSON.stringify(opts.palette)})`)
  if (opts.motion) sets.push(`localStorage.setItem('ccm.next.motion', ${JSON.stringify(opts.motion)})`)
  if (opts.reduceGlass !== undefined) sets.push(`localStorage.setItem('ccm.next.reduceGlass', '${opts.reduceGlass ? '1' : '0'}')`)
  return sets.join('; ')
}

async function setUiGeneration(evaluate, gen = 'next', opts = {}) {
  await evaluate(uiGenScript(gen, opts))
}

function uiGenFromArgs(argv = process.argv, env = process.env) {
  const flag = argv.find((a) => a.startsWith('--ui='))
  const gen = flag ? flag.slice(5) : env.CCM_E2E_UI || 'classic'
  return UI_GENERATIONS.includes(gen) ? gen : 'classic'
}

async function currentUiGeneration(evaluate) {
  return evaluate(`document.documentElement.dataset.ui || 'classic'`)
}

module.exports = { UI_GENERATIONS, uiGenScript, setUiGeneration, uiGenFromArgs, currentUiGeneration }
