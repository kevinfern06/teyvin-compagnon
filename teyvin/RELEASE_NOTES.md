Teyvin Companion is a build of [Irminsul](https://github.com/konkers/irminsul) v__UPSTREAM__ (commit `__SHA__`) that
shares its exports with [Teyvin](https://teyvin.app), a Genshin Impact planner, through a local relay on `127.0.0.1`.
Instructions are on [teyvin.app](https://teyvin.app/en/companion).

## Changes

- Works with game version 7.2. The account data is now recognized by its structure, so a new game version should no
  longer stop the companion. Characters and items added in 7.2 are skipped until Irminsul updates its game data.
- If the local port is still busy at startup, for example right after closing an older version, the relay now waits for
  it instead of giving up.

## Credits

Built on [Irminsul](https://github.com/konkers/irminsul) by Konkers and the projects it credits. MIT license, like
Irminsul. Unofficial fan project, not affiliated with HoYoverse or with the authors of Irminsul: please report issues
[here](https://github.com/kevinfern06/teyvin-compagnon/issues), not to them.
