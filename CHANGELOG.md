# Changelog

Every version is one build sprint. Newest first. Versions are tagged in git, and
each tag publishes a GitHub Release with downloads.

## [0.12.0] - 2026-10-05 - Seven monsters

Sprint 3 of 4: boss variety.

### Added
- **A different boss at every boss station on the loop**, each with its own name,
  colour and attacks:
  - **Ueno: THE STAMPEDE.** Stops, shakes, and a red line shows where it's about to
    charge. Then it rushes straight down that line, smashing through anything
    breakable, and a herd of walkers follows it in.
  - **Ikebukuro: THE BLOATED.** Slow, retching, lobbing arcs of acid that splash
    into glowing pools you don't want to stand in. It bursts into a ring of acid
    when it dies.
  - **Shinjuku: THE RUSH HOUR.** The original: ground slams that throw off runners.
  - **Shibuya: THE SCRAMBLE.** Quick lunges, and every few seconds a horn and a
    ring of runners converging on you from every side.
  - **Shinagawa: THE CONDUCTOR.** Fires fans of crackling sparks across the
    platform, and blows a whistle that whips every zombie into a faster frenzy.
  - **Tokyo: THE STATIONMASTER.** Slams, expanding shockwave rings you have to dash
    through (dashing makes you invulnerable for a moment), and it calls in brutes.
  - **Akihabara, the end: PATIENT ZERO.** Three phases as it's hurt: Stampede
    charges first, then sparks and acid, then shockwaves and scrambles, getting
    faster each time.
- Boss hazards (acid pools, sparks, shockwaves, lobbed acid) light up the dark.
- Arcade keeps The Rush Hour at Shinjuku, unchanged.

## [0.11.0] - 2026-10-05 - Coin lockers

Sprint 2 of 4: the permanent upgrade shop.

### Added
- **Coin lockers.** NEW GAME (and every RUN OVER) takes you to a wall of station
  coin lockers, where banked coins buy permanent upgrades that carry into every
  run. Each has several levels, with prices that climb:
  - TOUGHER: +15 max HP per level (5 levels)
  - STEADY HANDS: +10% damage (5)
  - QUICK TRIGGER: +8% fire rate (5)
  - RUNNING SHOES: +5% move speed (3)
  - DASH TRAINING: -10% dash cooldown (3)
  - GRENADE POUCH: -12% grenade cooldown (3)
  - LUCKY: +25% coins from smashing (3)
  - MAGNET: coins pull in from farther away (2)
  - HEAD START: start each run with a free upgrade card (2)
- **Skip the opening:** once you've seen it, a locker row lets you skip straight to
  Akihabara (pistol and flashlight in hand) on later runs. It's remembered.
- Upgrade levels and the skip choice are saved with the bank.
- Keyboard, mouse, controller and touch all work in the lockers: tap a locker to
  select it, tap again to buy.

## [0.10.0] - 2026-10-05 - Smash and bank

Sprint 1 of 4: roguelike runs and coins.

### Added
- **Breakable props in the loop:** vending machines (the jackpot), kiosks, bins,
  cardboard boxes and abandoned suitcases. Shoot them, or catch them in a grenade
  blast, until they break: glass shatters, debris flies, vending machines throw
  cans and sparks, and coins spill out across the floor.
- **Coins:** they bounce out, get pulled in when you're close, and count up bottom
  right. Breaking a prop also clears the space it took up.
- **The bank:** when a run ends (death or victory), its coins go into a bank that's
  saved between sessions (localStorage in the browser, a small file on desktop).
  The title screen shows your bank. Next sprint, the bank buys permanent upgrades
  at the start of each run.

### Changed
- **The loop is a roguelike:** dying ends the run. The new RUN OVER screen shows
  stations cleared, kills, coins earned and your bank, then it's back to the title
  for a fresh run. Arcade still lets you retry a station.

## [0.9.0] - 2026-10-05 - The loop plays like arcade

### Changed
- **The loop now plays just like Arcade.** Every station has a visible kill count
  (CLEARED 12/54); hit it and you get STATION CLEAR, the train arrives, and you
  board. Zombies hunt you from the moment you arrive, pouring in from the tunnels
  and the dark edges, with more of them all the way round the loop.
- **Upgrade cards are back, in the loop too:** after every ride (Miyake's call and
  the auto voice included), you pick one of three cards before the next station.
- **Bigger stations.** Small stops now have a concourse; medium ones are wider with
  a larger concourse; large and huge ones are wider still, with the underground
  halls.
- **No coins or looting.** Bodies stay where they fell but there's nothing to
  search, apart from the officer's flashlight in the opening. The coin counter is
  gone, and so is the vending-machine shop that was planned.
- Quotas start around 36-76 depending on station size and climb to a few hundred
  by the end of the loop; spawn rates and the number alive at once climb with them.

## [0.8.1] - 2026-10-05 - Arcade is back

### Changed
- **Arcade returns to the title menu, exactly as it was:** five stations from
  Akihabara to Shinjuku, the auto rifle, the flashlight from the start, and an
  upgrade card on every train ride. NEW GAME is the opening plus the loop.

## [0.8.0] - 2026-10-05 - The loop

### Changed
- **One game, one path.** Campaign and Arcade are gone from the menu: NEW GAME
  plays the opening at Kanda, then rolls straight into the main game. The upgrade
  cards are gone; vending machines will sell upgrades next.
- **The whole Yamanote loop.** 30 stations around, from Akihabara through Ueno,
  Ikebukuro, Shinjuku, Shibuya, Shinagawa, Tokyo and Kanda, and back to Akihabara
  (31 stops), with real names and JY station codes.
- **Stations come in four sizes**, built around the big Akihabara you liked:
  small (just platforms), medium (platforms and a concourse), large (Akihabara's
  size, with underground halls and passages), and huge (three tracks plus halls)
  for Ueno, Ikebukuro, Shinjuku, Shibuya, Shinagawa and Tokyo.
- **A hidden quota.** Each station has a number of kills it needs before it falls
  quiet and the train comes; you're not told how many. When it comes, a marker
  points you to the nearest open door.
- **It gets harder all the way round:** more wanderers, more hunters mixed in,
  runners early and brutes from the sixth stop, a little more health on each
  zombie, more flickering lights. Every station has bodies with coins on them.
- **Bosses at the big stations:** Ueno, Ikebukuro, Shinjuku, Shibuya, Shinagawa
  and Tokyo, and a final one back at Akihabara. Killing a mid-loop boss clears
  the station and calls the train; killing the last one ends the game.

### Added
- **Miyake.** On the train out of Kanda, the speaker crackles: "Hello? ...Hello?"
  It's Miyake, the last person left in the Yamanote line operations center, who
  has been watching you on the cameras and asks you to clear every station on
  the loop. She checks in on the radio at key points around the loop.
- **Rides between stations:** an empty car rattling through the tunnels (empty
  apart from those who never got off), the auto voice announcing the next stop
  and which side the doors open on. Tap, Enter or A skips ahead.
- The HUD shows which stop you're on (STOP 5 OF 31) and the next station.

## [0.7.3] - 2026-10-05 - Why won't they go down

### Changed
- **The officer's last stand:** he fires like a real pistol (steady pops, a pause
  every few shots), and the rounds clearly hit, but the zombies just soak them
  up and keep coming. "WHY WON'T YOU GO DOWN?!" "THERE'S TOO MANY..." Then, as
  they close in: "HEY KID, TAKE THIS! IT'S TOO DANGEROUS TO-" and he's dragged
  down mid-sentence. His throw falls short, landing between you and them.
- **The pistol now feels like a pistol:** slower rounds you can see travel, a
  slower trigger (3.5 shots a second), and less damage, so it takes three or four
  hits to drop one. The zombies that got him only become killable once you have
  his gun. Arcade mode keeps the original fast-firing rifle.

## [0.7.2] - 2026-10-05 - The figure in the corner

### Changed
- The dash scene moved to the far platform and plays out properly. Exploring up
  there, in front of the track entrances, you stop: someone's in the corner, under
  a flickering tube. "HELLO? ARE YOU OKAY?" They turn and rush you; the game
  freezes for the dash. You dash along the platform, and their lunge carries them
  straight through the gap and onto the tracks, where they just stop.
  "OH MY GOD... OH MY GOD..." Then they start back toward you. "WHAT THE F-" and
  the train takes them.
- A hint points you at the far platform if you haven't found it after a while.

## [0.7.1] - 2026-10-05 - Slower opening

### Changed
- The train ride lasts 20 seconds, then the doors open at Kanda.
- On the platform you shuffle along with the crowd at half walking speed (no
  dashing) toward the exits at the east end.
- The lights go first: a few seconds of flickering and electrical crackle, "WHAAAAA...?",
  and only then the quake, the panic, and the blackout.
- Waking up, you're down and out of control for a few seconds, then "WHAT HAPPENED?!",
  then you're back at full speed.

## [0.7.0] - 2026-10-05 - Underground

### Changed
- **Campaign Akihabara's lower level is now a subway concourse, not a maze.** Two
  rows of big halls (shuttered shops, poster walls, neon storefronts, square
  columns in the wide ones) open off a long central passage through wide
  doorways, with doorways between neighboring halls. Some halls open straight
  onto the passage.
- **No more escalator in the opening.** At Kanda the crowd heads east along the
  platform toward the exits, with a marker to follow. Partway there the quake
  hits: lights fail, people scream, some run and some go down, and you're knocked
  off your feet. You wake up where you fell, among the people you were following.
- **The officer's last stand moved to the top-right corner of the station** (the
  far platform's east end), and the zombies come out of the dark at the east end.
  It's slower now: they shamble in at about a third of the old speed, he fires
  about once a second, more of them arrive halfway through, and he can't be
  overrun before ten seconds have passed.

### Fixed
- **The open side of the train car looked bolted to a static ledge.** While the
  train moves, the space beyond the car's open side is now dark tunnel with
  motion-blurred streaks (wall seams, cables, the odd lamp) sliding past. It
  fades away as the train stops.

## [0.6.0] - 2026-10-05 - Campaign and Arcade

### Added
- **Title menu: Campaign or Arcade.** Arcade is the original run, unchanged: five
  stations from Akihabara to Shinjuku, the auto rifle, the flashlight from the
  start, and upgrade cards on every train ride.
- **Campaign** is the new story path:
  - The opening now happens at **Kanda** (JY02). The train goes straight there
    ("This train is bound for Ueno and Ikebukuro... the next station is Kanda").
  - **The officer is a real cutscene.** Walk near the middle platform after
    exploring and the camera pans over, letterbox bars come in, and you watch his
    last stand play out: his own flashlight cutting through the dark, more of them
    coming, "Kid! It's too dangerous here... take this!"
  - Picking up the pistol **pauses the game until you fire it once**.
  - Then a marker over the officer: **search his body** for coins and the flashlight.
  - Hold out at Kanda (fewer zombies than arcade, but still plenty) until a train
    comes, then ride it, with no upgrade cards, to...
  - **Akihabara, opened up.** One big level: the platforms up top and a maze of
    shuttered shops, poster walls and neon storefronts below. About 55 zombies are
    already shambling around, and they only come for you when they see you, hear
    gunshots, or get hit. Thirty bodies to **search for coins**, some empty.
- **Coins**, shown bottom right in the campaign. They're the start of the merchant
  economy.
- **The flashlight is an item.** Find it, then toggle it with L, D-pad up, or the
  LIGHT button on touch. The beam now starts at your hand, with a glow at the lens,
  instead of at your feet.
- Touch gets LIGHT and USE buttons. USE (F on keyboard, X on a controller)
  searches bodies and plays the arcade cabinet, which still sits in Kanda.

### Changed
- **The escalator**, rebuilt: brushed-steel treads with yellow edge lines, a comb
  plate, lit skirts, glass balustrades with black handrails, and more light on it.
  The ride is twice as long, with the quake building a third of the way up.
- The campaign has no upgrade cards; merchants will replace them.
- Retrying in the campaign restarts the level you died on, with what you carried in.

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
