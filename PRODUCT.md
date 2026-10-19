# Product

<!-- impeccable:product-schema 1 -->

## Platform

macos

## Users
One user: the developer who built it, a guitarist practising songs at their desk, sitting close to the screen. They load a recorded song, pull it apart into stems, and play along, mostly working on guitar solos.

## Product Purpose
Urdr is a personal, local-only take on Moises: it separates a song into stems, lets the player mute, solo and rebalance them, slow the song down, shift its pitch, loop a passage, and read the guitar part as tab. Success is a practice session where the player isolates the part they are learning, loops it at a tempo they can manage, and reads or corrects the tab without leaving the keyboard.

## Positioning
Everything runs on the player's own Mac: separation (Demucs six-stem), chords and beats (beat-this, madmom) and guitar tab transcription (Basic Pitch plus a hand-position fingering search). It adds what Moises lacks: an automatically generated guitar tab laid out Songsterr-style, playable through a guitar synth, and editable by hand because the generation is only a starting point.

## Operating Context
A native SwiftUI window on macOS, used at normal desk distance. The stem mixer owns the screen during practice (the user's choice); transport, chords and the tab score sit around it. Keyboard-first: single-key shortcuts for every frequent action, a Cmd+K command palette, a `?` shortcut overlay, Esc to close. The engine runs as a Python subprocess per job, so separation, analysis and transcription show progress and can take tens of seconds.

## Capabilities and Constraints
- Six stems: vocals, guitar, drums, bass, piano, other. Per-stem volume, mute, solo.
- Transport: play/pause, seek, skip 5 s, speed 50 to 200 %, pitch -12 to +12 semitones, A-B loop.
- Analysis: BPM, beats and downbeats, major/minor chord segments, current and next chord.
- Tab: generated from the guitar stem, shown as a measure score with chords, rhythm stems and a playback cursor; edit mode with a grid cursor, fret entry, string moves, delete, undo; synth playback with three GM tones (overdriven, clean, acoustic) replacing the original guitar.
- Library of songs cached under Application Support; songs open from a file picker or a launch argument.
- Not built yet: lyrics, waveform display, library browser.

## Brand Commitments
Name: Urdr. The user asked to take inspiration from Moises' design and layout.

## Evidence on Hand
No real songs on the machine. Demo content is GuitarSet (CC BY 4.0) solo recordings under `/tmp/urdr/guitarset`; one is loaded as `jazz-solo` in the app library. Do not fabricate song titles, artists or artwork.

## Product Principles
- The stems are the instrument: isolating and rebalancing parts must be immediate and obvious.
- Every action is one key away, and the screen always shows which key.
- Generated output is a draft the player can correct, never a final answer.
- Long engine jobs show honest progress and never block playback.
