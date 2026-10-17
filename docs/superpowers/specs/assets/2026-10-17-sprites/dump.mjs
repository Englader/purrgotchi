import { render, BREEDS } from './sprite.mjs'
const [mode] = process.argv.slice(2)
let cells = []
if (mode === 'breeds') for (const b of Object.keys(BREEDS)) cells.push(render({ breed: b, stage: 'adult', mood: 'alert' }))
if (mode === 'moods') for (const st of ['kitten','young','adult']) for (const m of ['calm','happy','alert','hungry','panic','sleep']) cells.push(render({ breed: 'Ginger', stage: st, mood: m }))
console.log(JSON.stringify({ cols: mode === 'breeds' ? 4 : 6, cells }))
