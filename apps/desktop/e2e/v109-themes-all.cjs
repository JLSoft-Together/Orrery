const fs = require('node:fs')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const KEPT = ['orrery-light', 'galaxy', 'orrery-dark']
const LEGACY = ['light', 'xcode-light', 'github-light', 'solarized-light', 'islands-light', 'dark', 'xcode-dark', 'github-dark', 'solarized-dark', 'islands-dark', 'monokai', 'dracula', 'material', 'one-dark', 'console']
const NEXT_MAIN = ['mist:light', 'mist:dark', 'graphite:light', 'mono:light', 'galaxy:dark', 'orrery:light', 'random', 'custom']
const NEXT_LEGACY = ['nebula:light', 'aurora:light', 'dawn:light', 'console:dark']
const rootTheme = `document.documentElement.dataset.theme ?? (document.documentElement.classList.contains('dark') ? 'dark' : 'light')`
const noHScroll = `document.documentElement.scrollWidth <= window.innerWidth + 1 && document.body.scrollWidth <= window.innerWidth + 1`

async function main() {
  const app = await launch({ ui: 'classic', env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep } = app
  const reload = async () => {
    await app.cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await sleep(400)
  }
  const openAppearance = async (probe) => {
    await evaluate(`${S}.get('ui').settingsOpen = true`)
    await waitFor(`!!document.querySelector('[data-settings-page="appearance"]')`, 10000, 'settings')
    await evaluate(`document.querySelector('[data-settings-page="appearance"]').click()`)
    await waitFor(probe, 10000, 'appearance')
    await sleep(300)
  }
  const closeSettings = async () => {
    await evaluate(`${S}.get('ui').settingsOpen = false`)
    await sleep(300)
  }
  const classicIds = `[...document.querySelectorAll('[data-theme-option]')].map((b) => b.dataset.themeOption)`
  const nextIds = `[...document.querySelectorAll('[data-palette-card]')].map((b) => b.dataset.paletteCard)`
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} })); localStorage.removeItem('ccm.themeViewAll'); localStorage.removeItem('ccm.next.paletteViewAll'); localStorage.setItem('ccm.theme','galaxy'); localStorage.setItem('ccm.themeMigration','4.1-v108')`)
    await reload()

    await openAppearance(`!!document.querySelector('[data-theme-option="orrery-dark"]')`)
    check('classic default lists only the kept themes', JSON.stringify(await evaluate(classicIds)) === JSON.stringify(KEPT), JSON.stringify(await evaluate(classicIds)))
    check('classic has a view all button, collapsed', await evaluate(`document.querySelector('[data-theme-view-all]')?.getAttribute('aria-expanded') === 'false'`))
    check('classic hides the older group', await evaluate(`!document.querySelector('[data-theme-legacy]')`))

    await evaluate(`document.querySelector('[data-theme-view-all]').click()`)
    await sleep(300)
    const opened = await evaluate(classicIds)
    check('classic view all lists kept then the 15 older themes', opened.length === 18 && KEPT.every((k, i) => opened[i] === k) && LEGACY.every((l) => opened.includes(l)), JSON.stringify(opened))
    check('classic older group is shown and button flips', await evaluate(`!!document.querySelector('[data-theme-legacy]') && document.querySelector('[data-theme-view-all]').getAttribute('aria-expanded') === 'true'`))
    check('classic view all state remembered', (await evaluate(`localStorage.getItem('ccm.themeViewAll')`)) === '1')
    check('classic settings no horizontal scroll', await evaluate(noHScroll))

    for (const id of LEGACY) {
      await evaluate(`document.querySelector('[data-theme-option="${id}"]').click()`)
      await sleep(250)
      const st = await evaluate(`({ root: ${rootTheme}, stored: ${S}.get('settings').theme, checked: document.querySelector('[data-theme-option="${id}"]')?.getAttribute('aria-checked') })`)
      check(`classic picks ${id}`, st.root === id && st.stored === id && st.checked === 'true', JSON.stringify(st))
    }

    await evaluate(`document.querySelector('[data-theme-option="dracula"]').click()`)
    await sleep(300)
    await closeSettings()
    await reload()
    check('classic older theme survives a restart', (await evaluate(rootTheme)) === 'dracula' && (await evaluate(`${S}.get('settings').theme`)) === 'dracula' && (await evaluate(`localStorage.getItem('ccm.theme')`)) === 'dracula')
    await evaluate(`localStorage.setItem('ccm.themeViewAll','0')`)
    await openAppearance(`!!document.querySelector('[data-theme-option="dracula"]')`)
    check('classic picker opens itself on an older theme', await evaluate(`!!document.querySelector('[data-theme-legacy]') && document.querySelector('[data-theme-option="dracula"]').getAttribute('aria-checked') === 'true'`))
    await evaluate(`document.querySelector('[data-theme-option="orrery-dark"]').click()`)
    await sleep(250)
    await evaluate(`localStorage.setItem('ccm.themeViewAll','0')`)
    await closeSettings()
    await reload()
    await openAppearance(`!!document.querySelector('[data-theme-option="orrery-dark"]')`)
    check('classic back on a kept theme, picker collapsed again', JSON.stringify(await evaluate(classicIds)) === JSON.stringify(KEPT))
    await closeSettings()

    await evaluate(`localStorage.setItem('ccm.theme','one-dark'); localStorage.setItem('ccm.themeMigration','4.0.2-galaxy'); localStorage.removeItem('ccm.themeViewAll')`)
    await reload()
    check('an older stored id is no longer forced to a kept theme', (await evaluate(rootTheme)) === 'one-dark')

    await evaluate(`localStorage.setItem('ccm.uiGeneration','next'); localStorage.setItem('ccm.uiMigration','4.1-next'); localStorage.setItem('ccm.nextTheme','mist'); localStorage.setItem('ccm.theme','orrery-light'); localStorage.removeItem('ccm.next.paletteViewAll')`)
    await reload()
    await openAppearance(`!!document.querySelector('[data-palette-card]')`)
    check('next default lists the main cards only', JSON.stringify(await evaluate(nextIds)) === JSON.stringify(NEXT_MAIN), JSON.stringify(await evaluate(nextIds)))
    check('next has a view all button, collapsed', await evaluate(`document.querySelector('[data-palette-view-all]')?.getAttribute('aria-expanded') === 'false' && !document.querySelector('[data-palette-legacy]')`))
    await evaluate(`document.querySelector('[data-palette-view-all]').click()`)
    await sleep(300)
    check('next view all adds the 4 older palettes after the main cards', JSON.stringify(await evaluate(nextIds)) === JSON.stringify([...NEXT_MAIN, ...NEXT_LEGACY]), JSON.stringify(await evaluate(nextIds)))
    check('next older group shown, state remembered', (await evaluate(`!!document.querySelector('[data-palette-legacy]')`)) && (await evaluate(`localStorage.getItem('ccm.next.paletteViewAll')`)) === '1')
    check('next settings no horizontal scroll', await evaluate(noHScroll))

    for (const card of NEXT_LEGACY) {
      const id = card.split(':')[0]
      await evaluate(`document.querySelector('[data-palette-card="${card}"]').click()`)
      await sleep(350)
      const st = await evaluate(`({ store: ${S}.get('uiGen').nextTheme, root: document.documentElement.dataset.nextTheme, checked: document.querySelector('[data-palette-card="${card}"]')?.getAttribute('aria-checked') })`)
      check(`next picks ${id}`, st.store === id && st.root === id && st.checked === 'true', JSON.stringify(st))
    }
    await evaluate(`document.querySelector('[data-palette-card="aurora:light"]').click()`)
    await sleep(300)
    await closeSettings()
    await reload()
    check('next older palette survives a restart', (await evaluate(`document.documentElement.dataset.nextTheme`)) === 'aurora' && (await evaluate(`${S}.get('uiGen').nextTheme`)) === 'aurora' && (await evaluate(`localStorage.getItem('ccm.nextTheme')`)) === 'aurora')
    await evaluate(`localStorage.setItem('ccm.next.paletteViewAll','0')`)
    await openAppearance(`!!document.querySelector('[data-palette-card]')`)
    check('next picker opens itself on an older palette', await evaluate(`!!document.querySelector('[data-palette-legacy]') && document.querySelector('[data-palette-card="aurora:light"]').getAttribute('aria-checked') === 'true'`))
    await closeSettings()
  } catch (e) {
    check('run', false, String(e?.message ?? e))
  } finally {
    if (results.some((r) => r.startsWith('FAIL'))) await shot('v109-fail').catch(() => {})
    app.stop()
    fs.writeFileSync(path.join(OUT, 'v109-results.txt'), results.join('\n'))
    const fails = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - fails}/${results.length} pass`)
    process.exit(fails ? 1 : 0)
  }
}
main()
