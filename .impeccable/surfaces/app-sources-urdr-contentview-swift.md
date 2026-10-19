---
version: 1
slug: "app-sources-urdr-contentview-swift"
primary_target: "app/Sources/Urdr/ContentView.swift"
related_targets: []
---

# Player window

Scope: the main Urdr window (idle, separating, mixer view, tab view). Mode: Operate. Task: isolate and rebalance stems, loop a passage at a slower tempo, read and correct the guitar tab, all from the keyboard.

## Direction contract

seed: none (direction pinned by the user: Moises-inspired, played straight)

THESIS: the stems are the instrument. A practice session is six coloured lanes you can see, silence and solo at a glance; the category default it refuses is a grey settings form of sliders.

OWN-WORLD: near-black ground with two lifted panel tones, white primary controls, one hue per stem (vocals rose, guitar amber, drums violet, bass cyan, piano green, other slate) used only for that stem's lane, icon and level; mint for playback and selection. SF Pro, tabular numerals for time and values.

STORY: open a song, watch separation progress, then see every part as a lane; mute what you play, loop the hard bar, slow it down, switch to the tab and fix it.

FIRST VIEWPORT: top bar (title, BPM, key-chipped Mixer/Tab switch); six lanes with header (icon, name, M/S, level) and waveform filling the width; chord lane on the same timeline; bottom transport bar (chord now/next, play, time, speed, pitch, loop, synth).

SIGNATURE: one playhead and loop region crossing every lane and the chord lane; muted lanes dim, soloed lanes glow in their hue.

RISK: six waveforms redrawn at 30 Hz; draw peaks once into a Canvas and move only the playhead.
