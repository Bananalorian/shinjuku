# Changelog

Every version is one build sprint. Newest first. Versions are tagged in git, and
each tag publishes a GitHub Release with downloads.

## [0.3.0] - 2026-10-04 - Controllers

### Added
- Controller support: left stick moves, right stick aims and fires, RT fires with
  soft lock-on, A/LB dash, RB/B grenade, Start pauses, Back mutes, d-pad drives menus.
  Desktop builds use gilrs (Xbox, PlayStation, Switch Pro and most others); the
  browser build reads the Gamepad API through a small JS plugin in `web/template.html`.
- Rumble on gunfire, dashes, hits, grenades, brute kills, the boss, and the arriving train.
- Pause (Start, Esc or P).
- Input modes: whatever you touched last (keyboard/mouse, touch, controller) drives
  aiming and the on-screen hints, so you can switch mid-run.
- Upgrade cards can be chosen with the d-pad or arrow keys.
- GitHub repository with CI: every push builds the browser version and desktop
  builds for Windows, macOS and Linux; version tags publish a Release.

### Fixed
- Sound on phones: audio now unlocks on the first finished tap (iOS requirement) and
  asks for a "playback" audio session so the iPhone silent switch doesn't mute it.
- Quick taps that started and ended within one frame were ignored on touch screens.

## [0.2.0] - 2026-10-04 - Sound

### Added
- A fully procedural sound bank (`src/synth.rs`), synthesized at startup with no audio
  files: gunshots, flesh hits, wet splats, grenade blasts, zombie groans shaped by vowel
  formants (walkers groan, runners shriek, brutes growl), the boss roar and slam.
- The train: two-tone horn, wheel clacks that slow as it brakes, brake squeal, air hiss,
  and the "pin-pon" door chime.
- An original departure-melody jingle when a station is cleared.
- Loops: a 120 bpm D-minor combat track, an ambient title pad, rail clacks for the ride
  between stations, and a 100 Hz fluorescent hum (Tokyo's grid runs at 50 Hz).
- `src/audio.rs`: variant picking, per-sound throttling, distance falloff, groans that
  scale with the size of the nearby horde, crossfaded loops, and M to mute.

## [0.1.0] - 2026-10-04 - First playable

### Added
- Isometric twin-stick shooter in Rust + macroquad, playable in the browser.
- Five stations up the Yamanote line: Akihabara, Ueno, Ikebukuro, Takadanobaba, and
  Shinjuku, each with its own layout, lighting, kill quota and zombie mix.
- Coded pixel art: every sprite, prop, the train, station walls with slanted name signs,
  and a hand-made 5x7 font are generated at startup. No image files.
- Render pipeline: light map with lamps, flashlight cone, muzzle flashes and glowing
  zombie eyes; post-process with dithered light bands, pixel-locked tilt-shift, bloom,
  haze, a miniature color grade, shockwaves, vignette and grain.
- Hordes of up to ~340 zombies with flow-field pathing; blood and scorch marks paint
  permanently into the floor.
- Walkers, runners, brutes, and the Shinjuku boss, The Rush Hour.
- The train arrives (and flattens anything on the track) when a station is cleared;
  board through a glowing door and pick one of three upgrades on the ride.
- Touch controls with twin virtual sticks and auto-aim.
