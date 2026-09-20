export interface IdColor {
  text: string
  bg: string
  border: string
}

// Muted, dark-theme-friendly accents. Blue and amber are the two the
// reference design used for its two secondaries; the rest extend the same
// family so any number of secondaries stay visually distinct.
const PALETTE: IdColor[] = [
  { text: '#8ab4f8', bg: 'rgba(138, 180, 248, 0.12)', border: 'rgba(138, 180, 248, 0.35)' },
  { text: '#e0b878', bg: 'rgba(224, 184, 120, 0.12)', border: 'rgba(224, 184, 120, 0.35)' },
  { text: '#c99bf0', bg: 'rgba(201, 155, 240, 0.12)', border: 'rgba(201, 155, 240, 0.35)' },
  { text: '#7fd6c8', bg: 'rgba(127, 214, 200, 0.12)', border: 'rgba(127, 214, 200, 0.35)' },
  { text: '#f0a0b8', bg: 'rgba(240, 160, 184, 0.12)', border: 'rgba(240, 160, 184, 0.35)' },
  { text: '#a8c97f', bg: 'rgba(168, 201, 127, 0.12)', border: 'rgba(168, 201, 127, 0.35)' },
]

export function colorForId(id: string): IdColor {
  let hash = 0
  for (let i = 0; i < id.length; i++) {
    hash = (hash * 31 + id.charCodeAt(i)) >>> 0
  }
  return PALETTE[hash % PALETTE.length]
}
