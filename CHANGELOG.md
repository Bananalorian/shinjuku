# Changelog

Every version is one build sprint. Newest first. Versions are tagged in git, and
each tag publishes a GitHub Release with downloads.

## [0.5.0] - 2026-10-05 - The last normal commute

### Added
- A playable opening, told through play rather than cutscenes:
  - **On the train.** A crowded Yamanote line car (about 70% full): commuters on
    the bench and holding straps, the car swaying side to side with everyone
    swaying with it, braking lurches, tunnel lights streaking past the windows,
    and announcements with the chime ("The next station is Kanda", a short stop,
    then "The next station is Akihabara. The doors on the left side will open").
  - **Akihabara.** The doors open (movement tutorial), the crowd shuffles off, and
    you follow it to a new escalator.
  - **The quake.** Halfway up, the ground shakes, the lights fail, people scream,
    and everything goes black.
  - **Waking up.** Red emergency lights, bodies on the platform, and someone on
    their knees being sick in the corner. They look up and charge; the game freezes
    and teaches you to dash. They bolt for the tracks as an out-of-service train
    comes through.
  - **The arcade.** A STAR COMMUTER cabinet glows in the dark: a small shooter you
    can actually play (free play for now; coins become currency later).
  - **The officer.** A police officer holds off the infected, gets overrun, and
    throws you his pistol: "It's too dangerous here... take this!" Kill what got
    him, emergency power comes back, and the Akihabara fight begins.
- Hold Esc, Tab or Start (or tap the corner on touch) to skip the opening.
- Interact: F on keyboard, X on a controller.
- Commuters, a kneeling pose, sitting and strap-holding poses, the officer, a
  pistol, an escalator, the arcade cabinet, and a train-car interior with seats,
  straps, hanging ads and door panels, all generated in code.
- Sounds: the quake, screams, retching, and arcade blips, coin and attract jingle.
- NPCs with their own pathfinding fields, pickups that arc through the air, and
  scripted trains that pass through without stopping.

### Changed
- You start with a pistol (infinite ammo, slower and harder-hitting) instead of
  the auto rifle. Upgrades still turn it into something meaner.
- A moving train now runs down anything on its track instead of shoving it aside.

## [0.4.0] - 2026-10-04 - Living stations

### Added
- Platform screen doors along every platform edge, the waist-high kind on the
  Yamanote line. Gaps sit exactly where the train doors stop, so the horde has to
  funnel through them; bullets fly over the panels and the boss smashes through.
  The flow field now understands walls between cells, not just blocked cells.
- Hanging signs over the platforms: station name boards and blinking amber LED
  departure boards ("FOR UENO / LAST TRAIN 00:12"). They fade when you walk behind.
- Painted floor detail baked into each station: numbered car-position markers at
  every door gap, big track numbers, puddles with glints, newspapers and flyers,
  cracks, old blood drag trails, dropped clear vinyl umbrellas, grime along walls.
- Clutter: a lit kiosk, abandoned suitcases, stacked boxes, construction barriers.
- Atmosphere: drifting mist that glows near lamps (anchored to the world, not the
  screen), failing tubes that spit sparks with an electric crackle, and stale air
  drifting out of the tunnels.
- Crows that peck around the platforms, scatter cawing when you get close or start
  shooting, and glide back down to the dead later.
- New synthesized sounds: crow caws and electrical zaps.
- Tilt-shift toggle: T on keyboard, Y on a controller. It stays on by default; the
  pause screen shows the current settings.

### Changed
- Benches, ticket gates, bins and other waist-high props no longer block bullets.
- Tall props (pillars, signs, vending machines, the kiosk) all fade when they hide
  the player, not just pillars.

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
  builds for Windows, macOS and Linux; version tags publish a Release. CI is pinned
  to Rust 1.91.1, the version the game is developed and tested with.

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
