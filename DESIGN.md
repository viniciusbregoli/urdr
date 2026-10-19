---
name: Urdr
description: A local stem-separation practice room for guitarists, Moises-inspired and keyboard-first.
colors:
  ground: "#0E0F11"
  panel: "#17191D"
  raised: "#212429"
  text: "#F2F5F7"
  secondary: "#9CA3B0"
  mint: "#5CF2D1"
  failure: "#FF6B66"
  stem-vocals: "#FF6B99"
  stem-guitar: "#FFB547"
  stem-drums: "#A68CFA"
  stem-bass: "#38D6F0"
  stem-piano: "#6EE8A1"
  stem-other: "#94A3B8"
typography:
  chord-readout:
    fontFamily: "SF Pro, -apple-system, system-ui"
    fontSize: "34px"
    fontWeight: 700
  lane-title:
    fontFamily: "SF Pro, -apple-system, system-ui"
    fontSize: "14px"
    fontWeight: 600
  body:
    fontFamily: "SF Pro, -apple-system, system-ui"
    fontSize: "13px"
    fontWeight: 500
  caption:
    fontFamily: "SF Pro, -apple-system, system-ui"
    fontSize: "11px"
    fontWeight: 600
  key-chip:
    fontFamily: "SF Pro Rounded, ui-rounded, system-ui"
    fontSize: "10px"
    fontWeight: 600
rounded:
  chip: "4px"
  pill: "6px"
  control: "8px"
  panel: "12px"
  overlay: "14px"
spacing:
  lane-gap: "6px"
  column-gap: "12px"
  pane: "16px"
  bar-x: "20px"
components:
  button-play:
    backgroundColor: "{colors.text}"
    textColor: "{colors.ground}"
    size: "48px"
  button-chrome:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    height: "30px"
    padding: "0 10px"
  button-chrome-on:
    backgroundColor: "{colors.text}"
    textColor: "{colors.ground}"
  button-loop-on:
    backgroundColor: "{colors.mint}"
    textColor: "{colors.ground}"
  pill-mute-on:
    backgroundColor: "{colors.text}"
    textColor: "{colors.ground}"
    rounded: "{rounded.pill}"
    width: "24px"
    height: "22px"
  pill-solo-on:
    backgroundColor: "{colors.stem-guitar}"
    textColor: "{colors.ground}"
    rounded: "{rounded.pill}"
  key-chip:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.secondary}"
    typography: "{typography.key-chip}"
    rounded: "{rounded.chip}"
    padding: "1.5px 5px"
  lane:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.panel}"
---

# Design System: Urdr

## Overview

**Creative North Star: "The Stems Are the Instrument"**

Urdr is a practice room, not a dashboard. The screen is near-black so six coloured lanes can carry the whole song: each stem owns one hue and uses it nowhere else, and a single mint playhead and loop region cut across every lane and the chord lane so the player always sees where they are in all parts at once. White is reserved for the few controls that matter most (play, mute, the sounding chord).

The layout follows Moises: lanes fill the window, a bottom transport bar holds the chord readout, the play button and the practice controls, and the native toolbar holds the song title and the Mixer/Tab switch. The tab view swaps the lanes for a Songsterr-style score on the same ground.

**Key Characteristics:**
- Near-black ground with two lifted panel tones
- One hue per stem, confined to that stem's lane, icon, level and solo state
- Mint marks playback and selection, nothing else
- White primary controls
- A visible key chip beside every control

## Colors

### Primary
- **Playback mint** (`mint`): the playhead, the loop region (14 % fill, 70 % edges when active), the active loop button, the tab cursor, sounding fret digits and the edit selection box.

### Secondary
Six stem hues, one per stem: vocals rose `#FF6B99`, guitar amber `#FFB547`, drums violet `#A68CFA`, bass cyan `#38D6F0`, piano green `#6EE8A1`, other slate `#94A3B8`. Each colours its lane's waveform (38 % unplayed, full played), its header icon disc, its level slider and its solo wash.

### Tertiary
- **Failure** (`failure`): error text only (failed open, failed analysis or transcription).

### Neutral
- **Ground** `#0E0F11` window, **Panel** `#17191D` lanes, headers, score and transport bar, **Raised** `#212429` controls and chips, hairlines at white 8 %.
- **Text** `#F2F5F7` and **Secondary** `#9CA3B0` (about 7:1 on ground).

### Named Rules
**The One Hue Per Stem Rule.** A stem's hue appears only on that stem's lane, icon, level slider and solo state. Never reuse it for chrome or state.

**The Mint Means Now Rule.** Mint is the playhead, the loop region, the active loop button, the tab cursor, sounding notes and the edit selection. If it is not about playback position or selection, it is not mint.

## Typography

SF Pro throughout (a working screen; the system face is the right tool). Tabular numerals for every changing number: time, speed, pitch, BPM.

### Hierarchy
- **Chord readout** 34 bold: the sounding chord, the only large type during practice.
- **Lane title** 14 semibold: stem names, song title, pane titles.
- **Body** 13 medium: controls, descriptions, overlay rows.
- **Caption** 11 semibold secondary: control group labels (Speed, Pitch, Loop, Guitar).
- **Key chip** 10 semibold SF Rounded.

### Named Rules
**The Key On Screen Rule.** Every control shows its key as a chip or names it in its tooltip; a control without a key is unfinished.

## Layout

The mixer is a fixed 248 pt header column beside a timeline column; lanes share the height equally (minimum 52 pt) with 6 pt gaps, and a 40 pt chord lane closes the stack. Everything time-based uses one x scale: x = time / duration * width. The transport bar is 84 pt tall in three columns, chord left, play centred on the window's centre line, practice controls right.

## Elevation & Depth

Flat. Surfaces separate by tone (ground, panel, raised) and 1 px white 8 % hairlines on the toolbar and transport edges.

### Named Rules
**The Flat Chrome Rule.** Surfaces are separated by tone, not by shadows. The only shadow belongs to floating overlays (palette, shortcuts): 0 12 30 black 50 % over a 55 % black scrim.

## Shapes

Radii: 4 chips, 6 mute/solo pills, 8 controls, 12 lanes and panels, 14 overlays. The play button and stem icon discs are circles.

## Components

### Buttons
- **Play**: 48 pt white circle, black play/pause glyph, centred in the transport bar.
- **Chrome**: raised fill at 70 %, 30 pt tall, 8 radius; on state fills white with black text, the loop button fills mint.

### Chips
- **Key chip**: raised fill, hairline border, secondary text, SF Rounded 10 semibold, beside every control and in the shortcut overlay.
- **Mute/solo pills**: 24 x 22; mute fills white, solo fills the stem hue.

### Cards / Containers
- **Lane**: panel fill, 12 radius. A soloed lane and its header get a 12 % wash and a 1 px stroke in the stem hue; a lane that cannot be heard drops to 70 % with a grey icon, grey level and dim waveform.

### Navigation
- Native unified toolbar on ground: song title and BPM leading, a segmented Mixer/Tab switch with its V chip in the centre, Open, command palette and shortcut buttons trailing, each with its chip.

### Stem lanes on a shared timeline (signature)
One mint playhead (2 pt line, 10 pt knob) and one loop region cross all lanes and the chord lane. Click seeks; dragging across the lanes marks a loop.

## Do's and Don'ts

### Do:
- Do keep the timeline shared: anything time-based (lanes, chords, loop) uses the same x scale and the same playhead.
- Do dim a lane that cannot be heard (muted, or not soloed while another lane is): icon, name, waveform and level together.
- Do use tabular numerals for time, speed, pitch and BPM.

### Don't:
- Don't introduce a second accent; state colours beyond mint are limited to failure red.
- Don't paint chrome with stem hues.
- Don't add shadows to panels or lanes.
- Don't use monospace for key chips; O and 0 must stay distinguishable.
