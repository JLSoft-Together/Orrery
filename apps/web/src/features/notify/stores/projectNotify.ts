import { defineStore } from 'pinia'
import { ref } from 'vue'
import { levelAllows, levelFor, parseLevels, withLevel, type NotifyLevel } from '../projectLevel'

const KEY = 'ccm.notify.projects'

function load(): Record<string, NotifyLevel> {
  try {
    return parseLevels(localStorage.getItem(KEY))
  } catch {
    return {}
  }
}

export const useProjectNotifyStore = defineStore('projectNotify', () => {
  const levels = ref<Record<string, NotifyLevel>>(load())

  function levelOf(cwd: string | undefined | null): NotifyLevel {
    return levelFor(levels.value, cwd)
  }

  function set(root: string, level: NotifyLevel): void {
    levels.value = withLevel(levels.value, root, level)
    try {
      localStorage.setItem(KEY, JSON.stringify(levels.value))
    } catch {
      return
    }
  }

  function allows(cwd: string | undefined | null, kind: string): boolean {
    return levelAllows(levelFor(levels.value, cwd), kind)
  }

  return { levels, levelOf, set, allows }
})
