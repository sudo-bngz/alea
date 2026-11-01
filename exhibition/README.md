# ALEA — three works, endless editions

An exhibition of generative works written in alea, a language where a program
describes a family of artworks and every run draws a new one: an *edition*.
Each image here is one edition, named by its seed. The same seed draws the
same image again, exactly; any other seed draws a work no one has seen.

| Work | File | What it is |
|---|---|---|
| **Isohypses** | `isohypses.alea` | Maps of places that don't exist: contour lines over an imaginary relief, layer tints, a hatched sea, towns, and the cartouche of a survey sheet. |
| **Nocturne** | `nocturne.alea` | The moon over a sea at night, its path broken into glints by waves in perspective. Also a Reel: clouds drift and the water moves, in a seamless loop, with its own soundtrack. |
| **Rosace** | `rosace.alea` | A rose window of stained glass: panes from a noise field, lead drawn as contour lines, tracery folded into the window's symmetry. |
| *Opening card* | `exhibition.alea` | A triptych of the three works, each called as a component. |
| **Culture** | `culture.alea` | A petri dish: colonies seeded at random grow by reaction–diffusion, two kinds of life side by side. Also a Reel: 72 hours of incubation in 20 seconds. |
| **Correspondances** | `correspondances.alea` (+ `correspondances/`) | The plan of a metro for a Paris that was never built: the real city's grammar, a new geometry every edition, and a Reel of one day of service. |
| **Atardecer** | `atardecer.alea` | A sunset on the sea, the sun a record setting into it: grooves, a turning sheen, a label with the side it plays, the reflection broken into slats, palms in silhouette, in washed-out colours. Also a Reel of slow, lofi house. |
| **Kikai** (機械図) | `kikai.alea` (+ `fonts/`) | The technical drawing of an analog machine that never existed: a cassette deck, a radio-cassette, a portable radio or a rhythm machine, in off-white line on washed charcoal, labelled in Japanese like a service manual. Each edition draws another machine. Also a Reel: the machine plays a techno track in the spirit of Detroit, with its acid line written below. |
| **Parasite** | `parasite.alea` (+ `parasite/`) | A broadcast being lost: snow, someone who appears in it made of the same blocks, a desktop filling with windows, a test card counting down, the figure again. Five works on a timeline, crossed with glitches, a morph and a fade, in a loop; a drone of pulses and a burst of static at every cut. |
| **Requête** | `requete.alea` (+ `requete/`) | The life of one request, layer by layer: the address typed, the name resolved down a tree, the handshake and the lock, packets finding their way across a grid of routers, bits and pulses of light on the wire, and the page building itself. Six works on a timeline, in a loop, each with its own sound. |
| **Signal** | `signal.alea` (+ `signal/`) | The path of a sound, from the oscillator to the room: DCO, noise, LFO, envelope, high-pass, filter, VCA and chorus, delay, the mix, the compressor, the lathe, the vinyl, the stylus, the speaker, the room. Fifteen works in twenty-one fast cuts, a minute, in a loop; a Detroit techno track built stage by stage with the chain. |
| **Hana** (花) | `hana.alea` (+ `hana/`) | An electronic flower, from seed to seed, in a network: a seed calling in the grid, roots along its links, a stem of square cells, a bud of nested squares, the bloom of square petals, the seeds leaving along the links and on the wind, flowers everywhere, the petals falling apart into grain. Forty-one cuts in a minute, glitches drawn anew every edition, an IDM track in the Boards of Canada way. |
| **Babel** | `babel.alea` (+ `babel/`) | The library that holds every book: pages of a script of twenty-five symbols rewritten on every tick, a book in three dimensions turning in the dark, the shelves, a hexagonal gallery from above, the hallways, the shaft and its stair, the mirror, and one long rise out of a gallery until the floor of galleries runs to the horizon. Forty-one cuts in a minute, a dark choir and a broken beat. |
| **Kairo** (回路) | `kairo.alea` (+ `fonts/`) | After Kikai, a modular synthesizer that never existed: two rows of modules racked in another order each edition, patched with hanging cables. Also a Reel without drums: a slow piano waltz over a deep drone, the melody written on a staff as it plays. |

## Replaying the editions

```sh
alea isohypses.alea --seed 0xa1ea03          # writes the PNG of that edition
alea nocturne.alea  --seed 0xa1ea06          # the image, and the 1080×1920 Reel (needs ffmpeg)
alea exhibition.alea --seed 0xa1ea43         # the opening card
alea correspondances.alea --seed 0xa1ea00    # the plan, and the 1080×1920 Reel of its day
alea culture.alea --seed 0xc06               # the plate, and the Reel of its growth (2 to 3 minutes of growing first)
alea kairo.alea --seed 0xa1ea01              # the rack, and the 1080×1920 Reel of its waltz (about 30 minutes)
alea babel.alea --seed 0xa1ea01              # the cover, and the 1080×1920 Reel: 60 s, in a loop (about 20 minutes)
alea hana.alea --seed 0xa1ea01               # the cover, and the 1080×1920 Reel: 60 s, in a loop (about 30 minutes)
alea signal.alea --seed 0xa1ea01             # the 1080×1920 Reel: 60 s, in a loop
alea requete.alea --seed 0xa1ea01            # the 1080×1920 Reel: 28.5 s, in a loop
alea parasite.alea --seed 0xa1ea01          # the 1080×1920 Reel: 24 s, in a loop
alea kikai.alea --seed 0xa1ea05              # the drawing, and the 1080×1920 Reel of the machine playing (about 10 minutes)
```

The posted editions:

| Post | Editions (in carousel order) |
|---|---|
| Isohypses | 0xa1ea03 · 0xa1ea0a · 0xa1ea01 · 0xa1ea06 · 0xa1ea0d · 0xa1ea07 |
| Nocturne | 0xa1ea06 · 0xa1ea04 · 0xa1ea09 · 0xa1ea08 · 0xa1ea0c · 0xa1ea0b |
| Rosace | 0xa1ea0e · 0xa1ea05 · 0xa1ea07 · 0xa1ea0b · 0xa1ea00 · 0xa1ea0f |
| Reel | Nocturne 0xa1ea06 |
| Opening card | Exhibition 0xa1ea43 |
| Correspondances | 0xa1ea00 · 0xa1ea01 · 0xa1ea05 · 0xa1ea03 · 0xa1ea08 · 0xa1ea0b |
| Correspondances Reel | 0xa1ea18 |
| Culture | 0xc00 · 0xc05 · 0xc06 · 0xc0b · 0xc0a · 0xc01 |
| Culture Reel | 0xc06 |
| Kikai | 0xa1ea05 · 0xa1ea04 · 0xa1ea08 · 0xa1ea01 · 0xa1ea0a · 0xa1ea0f |
| Kikai Reel | 0xa1ea05 |
| Atardecer | 0xa1ea01 · 0xa1ea03 · 0xa1ea04 · 0xa1ea02 · 0xa1ea06 · 0xa1ea07 |
| Atardecer Reel | 0xa1ea01 |
| Kairo | 0xa1ea01 · 0xa1ea00 · 0xa1ea02 · 0xa1ea03 · 0xa1ea04 · 0xa1ea06 |
| Kairo Reel | 0xa1ea01 |
| Parasite Reel | 0xa1ea01 |
| Requête Reel | 0xa1ea01 |
| Signal Reel | 0xa1ea01 |
| Hana Reel | 0xa1ea01 |
| Babel Reel | 0xa1ea01 |

All images are 1080 × 1350 (4:5, Instagram's portrait format); the Reel is
1080 × 1920 (9:16), 15 seconds, with sound.

## Posting plan

Instagram shows the newest post first, so the opening card goes up last (or is
pinned) to lead the grid. Files in `instagram/` are numbered in posting order.

| # | Post | Files |
|---|---|---|
| 1 | Isohypses, carousel of 6 | `1-isohypses-*.jpg` |
| 2 | Nocturne, carousel of 6 | `2-nocturne-*.jpg` |
| 3 | Rosace, carousel of 6 | `3-rosace-*.jpg` |
| 4 | Behind the editions: the code, carousel of 6 | `4-behind-the-editions-*.jpg` |
| 5 | Nocturne, Reel (cover: `5-nocturne-reel-cover.jpg`) | `5-nocturne-reel.mp4` |
| 6 | Opening card (pin it) | `6-alea-card.jpg` |

The profile grid then reads, newest first:
`card · reel · code` on the first row, `rosace · nocturne · isohypses` on the
second (see `instagram/grid-preview.png`).

---

## Captions

### 1 · Isohypses

**EN**

ISOHYPSES — maps of places that don't exist.

Every map is one edition of a single program. It invents a relief, then reads
it the way a cartographer would: contour lines every level, a heavier one
every fifth, tints that darken with altitude, a hatched sea, a few towns, a
scale. Six editions, six countries. Run it again and there's a seventh.

Editions 0xa1ea03 · 0xa1ea0a · 0xa1ea01 · 0xa1ea06 · 0xa1ea0d · 0xa1ea07
Written in alea.

**FR**

ISOHYPSES — cartes de lieux qui n'existent pas.

Chaque carte est une édition d'un même programme. Il invente un relief, puis
le lit comme le ferait un cartographe : une courbe de niveau par palier, une
plus épaisse toutes les cinq, des teintes qui foncent avec l'altitude, une mer
hachurée, quelques villes, une échelle. Six éditions, six pays. Relancez-le,
il y en aura un septième.

Éditions 0xa1ea03 · 0xa1ea0a · 0xa1ea01 · 0xa1ea06 · 0xa1ea0d · 0xa1ea07
Écrit en alea.

### 2 · Nocturne

**EN**

NOCTURNE — the moon over a sea at night.

A sky that darkens upwards, a field of stars, a moon (full, gibbous, or a
crescent) and, on the water, its path broken into glints by waves that grow
finer towards the horizon. One program; each image is the night it drew.

Editions 0xa1ea06 · 0xa1ea04 · 0xa1ea09 · 0xa1ea08 · 0xa1ea0c · 0xa1ea0b
Written in alea.

**FR**

NOCTURNE — la lune sur la mer, la nuit.

Un ciel qui s'assombrit vers le haut, un champ d'étoiles, une lune (pleine,
gibbeuse ou en croissant) et, sur l'eau, son chemin brisé en éclats par des
vagues de plus en plus fines vers l'horizon. Un seul programme ; chaque image
est la nuit qu'il a tirée.

Éditions 0xa1ea06 · 0xa1ea04 · 0xa1ea09 · 0xa1ea08 · 0xa1ea0c · 0xa1ea0b
Écrit en alea.

### 3 · Rosace

**EN**

ROSACE — rose windows that were never built.

The glass is drawn from a field of noise: each band of it a pane, the lead
between them drawn like contour lines. Rings of tracery, petals and small
round windows organise it, folded into a symmetry of 6, 8, 10, 12 or 16. The
light comes from behind, brightest at the heart.

Editions 0xa1ea0e · 0xa1ea05 · 0xa1ea07 · 0xa1ea0b · 0xa1ea00 · 0xa1ea0f
Written in alea.

**FR**

ROSACE — des rosaces qui n'ont jamais été construites.

Le verre naît d'un champ de bruit : chacune de ses bandes devient un vitrail,
et le plomb entre elles est tracé comme des courbes de niveau. Des cercles de
remplage, des pétales et des oculi l'organisent, repliés selon une symétrie
d'ordre 6, 8, 10, 12 ou 16. La lumière vient de derrière, plus vive au cœur.

Éditions 0xa1ea0e · 0xa1ea05 · 0xa1ea07 · 0xa1ea0b · 0xa1ea00 · 0xa1ea0f
Écrit en alea.

### 4 · Behind the editions

**EN**

Behind the editions: the code.

Each work in this exhibition is a short program in alea, a language I'm
building for generative art. A program doesn't describe one image; it
describes every image it could make. `~between(0.36, 0.5)` is a sea level not
yet decided; `choose { 6 | 8 | 12 }` is a symmetry not yet chosen. Each run
decides, and that's an edition.

Swipe for the full code of Isohypses, Nocturne and Rosace.

**FR**

Derrière les éditions : le code.

Chaque œuvre de cette exposition est un court programme en alea, un langage
que je construis pour l'art génératif. Un programme ne décrit pas une image ;
il décrit toutes celles qu'il pourrait faire. `~between(0.36, 0.5)` est un
niveau de la mer pas encore décidé ; `choose { 6 | 8 | 12 }` une symétrie pas
encore choisie. Chaque exécution décide : c'est une édition.

Faites défiler pour le code complet d'Isohypses, Nocturne et Rosace.

### 5 · Nocturne (Reel)

**EN**

NOCTURNE, in motion. Clouds drift, the moon's path shivers on the water, and
the sound is drawn from the same scene: low strings that follow the sky,
glass chimes, and the sea, in stereo. Fifteen seconds that loop forever. Edition 0xa1ea06.

**FR**

NOCTURNE, en mouvement. Les nuages passent, le chemin de lune frissonne sur
l'eau, et le son est tiré de la même scène : des cordes graves qui suivent le
ciel, des carillons de verre, et la mer, en stéréo. Quinze secondes qui bouclent sans fin. Édition 0xa1ea06.

### 6 · Opening card

**EN**

ALEA — three works, endless editions.

An exhibition of generative art written in alea, a language where every run
of a program draws a new artwork. Isohypses, Nocturne and Rosace: each post is
a handful of editions; the programs could make millions more. Even this card
is an edition: its three panels are the works themselves, called by a fourth
program.

**FR**

ALEA — trois œuvres, des éditions sans fin.

Une exposition d'art génératif écrite en alea, un langage où chaque exécution
d'un programme tire une œuvre nouvelle. Isohypses, Nocturne et Rosace : chaque
publication en montre quelques éditions ; les programmes pourraient en faire
des millions d'autres. Même cette affiche est une édition : ses trois panneaux
sont les œuvres elles-mêmes, appelées par un quatrième programme.

## Second room: Correspondances

A new work, posted after the opening, or in Nocturne's place (posts 2 and 5).

| # | Post | Files |
|---|---|---|
| 7 | Correspondances, carousel of 6 | `7-correspondances-*.jpg` |
| 8 | Correspondances, Reel (20 s, a day of service) | `8-correspondances-reel.mp4` (cover: `8-correspondances-reel-cover.jpg`) |

**How it is made.** The interchanges sit near where they really are
(Étoile, Saint-Lazare, Châtelet, République, Nation…), jittered and snapped to
the plan's grid. Each of the fourteen lines goes through the interchanges it
really serves, but between two of them it runs as a schematic map draws it:
a diagonal and a straight stretch, the order chosen by chance. Under the
network: the Seine with its two islands, the ring road, the woods of Boulogne
and Vincennes, and the arrondissements as a faint spiral. In the Reel, `t` is
one day: no trains at night, the first ones at dawn, all of them at the rush
hours, a chime for each arrival.

### 7 · Correspondances

**EN**

CORRESPONDANCES — a metro for a Paris that was never built.

Every plan keeps the city's grammar: the Seine bending north through the
middle, the ring road, the woods on either side, the great interchanges, and
which lines meet where. What changes is the geometry: where Châtelet falls,
which way each line bends, how the river winds. Fourteen lines, a different
Paris every time.

Editions 0xa1ea00 · 0xa1ea01 · 0xa1ea05 · 0xa1ea03 · 0xa1ea08 · 0xa1ea0b
Written in alea.

**FR**

CORRESPONDANCES — le métro d'un Paris qui n'a jamais été construit.

Chaque plan garde la grammaire de la ville : la Seine qui remonte vers le nord
au milieu, le périphérique, les bois de part et d'autre, les grandes
correspondances, et quelles lignes s'y croisent. Ce qui change, c'est la
géométrie : où tombe Châtelet, de quel côté chaque ligne tourne, comment le
fleuve serpente. Quatorze lignes, un Paris différent à chaque fois.

Éditions 0xa1ea00 · 0xa1ea01 · 0xa1ea05 · 0xa1ea03 · 0xa1ea08 · 0xa1ea0b
Écrit en alea.

### 8 · Correspondances (Reel)

**EN**

CORRESPONDANCES, one day of service in twenty seconds. The night is empty;
the first trains come out at dawn; at the rush hours every line is full; then
the last trains, and the night again. Each arrival rings on a kalimba, the
air moves in the tunnels, the rails clatter at the rush hour. The loop never ends,
like the network. Edition 0xa1ea18.

**FR**

CORRESPONDANCES, une journée d'exploitation en vingt secondes. La nuit est
vide ; les premières rames sortent à l'aube ; aux heures de pointe, toutes les
lignes sont pleines ; puis les derniers métros, et la nuit à nouveau. Chaque
arrivée sonne au kalimba, l'air circule dans les tunnels, les rails claquent
aux heures de pointe. La boucle ne finit jamais, comme le réseau. Édition 0xa1ea18.

## Third room: Culture

| # | Post | Files |
|---|---|---|
| 9 | Culture, carousel of 6 | `9-culture-*.jpg` |
| 10 | Culture, Reel (20 s, 72 hours of growth) | `10-culture-reel.mp4` (cover: `10-culture-reel-cover.jpg`) |

**How it is made.** Culture is the first work that uses `grow`: fields with a
memory, computed generation by generation on a grid of 256 × 256 cells. Two
chemicals, food and life, react and spread for 6000 generations (a Gray–Scott
reaction–diffusion). A few drops of life are seeded at random in the dish. The
chemistry differs across the dish: two regimes are drawn for each edition
(cells that divide, coral, maze, fingerprint, holes, spots) and blended along
a map of noise, so two kinds of growth meet. A third grown field keeps a
faint stain wherever life has been. The light is computed from the slope of
the living layer, as on a wet surface.

### 9 · Culture

**EN**

CULTURE — what grows in a petri dish.

A few drops of life, placed by chance. Then two chemicals react and spread,
generation after generation: spots that divide like cells, mazes, corals,
fingerprints. The chemistry changes across the dish, so two kinds of life
grow side by side and meet. Nothing here is drawn; all of it is grown.

Editions 0xc00 · 0xc05 · 0xc06 · 0xc0b · 0xc0a · 0xc01
Written in alea.

**FR**

CULTURE — ce qui pousse dans une boîte de Petri.

Quelques gouttes de vie, posées au hasard. Puis deux substances réagissent et
se diffusent, génération après génération : des taches qui se divisent comme
des cellules, des labyrinthes, des coraux, des empreintes. La chimie change
d'un bord à l'autre de la boîte, alors deux formes de vie poussent côte à côte
et se rencontrent. Rien n'est dessiné ici ; tout a poussé.

Éditions 0xc00 · 0xc05 · 0xc06 · 0xc0b · 0xc0a · 0xc01
Écrit en alea.

### 10 · Culture (Reel)

**EN**

CULTURE, seventy-two hours in twenty seconds. The drops swell into rings, the
rings meet and fold into a labyrinth, and in the middle a colony of cells
keeps dividing. The sound follows the growth: a choir whose voice opens as
life spreads, drops of glass, the wet crackle of the culture. Edition 0xc06.

**FR**

CULTURE, soixante-douze heures en vingt secondes. Les gouttes gonflent en
anneaux, les anneaux se rejoignent et se replient en labyrinthe, et au milieu
une colonie de cellules ne cesse de se diviser. Le son suit la croissance :
un chœur dont la voix s'ouvre à mesure que la vie s'étend, des gouttes de
verre, le crépitement humide de la culture. Édition 0xc06.

## The wall

`wall.alea` shows the exhibition at once: Isohypses, Correspondances, Rosace
and Culture, each called as a component, painted in its own colours
(`paint`) and given its own screen in a mosaic (`mosaic`). In the video all
four run together: the map's relief drifts, the trains run, the culture grows.

| # | Post | Files |
|---|---|---|
| 11 | The wall, video (15 s, 4:5, with sound) | `11-wall.mp4` (still: `11-wall-0xa1ea18.jpg`) |

```sh
alea wall.alea --seed 0xa1ea18      # about 4 minutes: Culture grows first
```

**EN** — ALEA, the wall: four works, four screens, one edition each. Every
screen is a program running at the same moment.

**FR** — ALEA, le mur : quatre œuvres, quatre écrans, une édition chacune.
Chaque écran est un programme qui tourne au même instant.

## Fourth room: Kikai

| # | Post | Files |
|---|---|---|
| 12 | Kikai, carousel of 6 | `12-kikai-*-0x*.jpg` (0xa1ea05 · 0xa1ea04 · 0xa1ea08 · 0xa1ea01 · 0xa1ea0a · 0xa1ea0f) |
| 12 | Kikai, Reel (15 s, eight bars at 128 BPM) | `12-kikai-reel.mp4` (cover: `12-kikai-reel-cover.jpg`) |

**How it is made.** The drawing is the still, and every motion is zero at
`t = 0`, so the print is the Reel's first frame. `t` is eight bars of four
beats: the kick is a pulse that rises in a frame and fades, and the speakers
swell with it, the VU needle swings with a slower one, the reels turn at two
speeds, the step lamp runs over the sixteen pads. The track is written in alea
too, as synths: a drum machine (a sine that falls in pitch for the kick,
filtered noise for the claps and hats) and a bass synthesizer, a saw through a
resonant filter, the acid line. That line is one bar drawn for each edition,
and it is drawn under the machine, step by step, as it plays. In the last four
bars come the stabs of Detroit, a minor ninth and a major ninth; in the last two
beats the kick stops and the picture turns to negative, once.

### 12 · Kikai (Reel)

**EN**

KIKAI, in motion. A rhythm machine that never existed plays a track that
never existed either: four on the floor at 128 BPM, an acid line drawn
by chance (the one written under the machine is the one you hear), and in the
second half the minor chords of Detroit. Drawn, composed and synthesized by
the same program. Edition 0xa1ea05.

**FR**

KIKAI, en mouvement. Une boîte à rythmes qui n'a jamais existé joue un
morceau qui n'a jamais existé non plus : quatre temps à 128 BPM, une ligne
acid tirée au hasard (celle qui est écrite sous la machine est celle qu'on
entend) et, dans la seconde moitié, les accords mineurs de Detroit. Dessiné,
composé et synthétisé par le même programme. Édition 0xa1ea05.

## Fifth room: Atardecer

| # | Post | Files |
|---|---|---|
| 13 | Atardecer, carousel of 6 | `13-atardecer-*-0x*.jpg` (0xa1ea01 · 0xa1ea03 · 0xa1ea04 · 0xa1ea02 · 0xa1ea06 · 0xa1ea07) |
| 13 | Atardecer, Reel (16.5 s, eight bars at 116 BPM) | `13-atardecer-reel.mp4` (cover: `13-atardecer-reel-cover.jpg`) |

**How it is made.** The sun is written as a function of the point, not as a
shape, so the sea can draw it again upside down, shifted row by row by the
ripples into slats. It turns at 33 rpm (nine turns in the loop), its grooves
a fine sine of the distance, the light on them a sine of twice the angle. The
palms are paths: a trunk that leans, fronds that rise and droop, leaflets
hanging from them, swaying around the crown. The music is lofi house: a round
kick, a soft snare, shakers and congas on a swung grid, a rolling bass on
minor ninths, an arpeggio that opens, and a lead's long notes from the fifth
bar. All of it runs through a slow wobble of pitch, like an old tape, with the
crackle of a record.

### 13 · Atardecer (Reel)

**EN**

ATARDECER, a record setting into the sea. It turns at 33 rpm, the sea breaks
its reflection into slats, the palms sway. The track is lofi house at 116 BPM:
congas, a rolling bass, an arpeggio over minor ninths, the wobble of an old
tape. Drawn, composed and synthesized by the same program. Edition 0xa1ea01.

**FR**

ATARDECER, un disque qui se couche dans la mer. Il tourne à 33 tours, la mer
brise son reflet en lamelles, les palmiers se balancent. Le morceau est de la
house lofi à 116 BPM : des congas, une basse qui roule, un arpège sur des
accords de neuvième mineure, le pleurage d'une vieille cassette. Dessiné,
composé et synthétisé par le même programme. Édition 0xa1ea01.
## Sixth room: Kairo

| # | Post | Files |
|---|---|---|
| 14 | Kairo, carousel of 6 | `14-kairo-*-0x*.jpg` (0xa1ea01 · 0xa1ea00 · 0xa1ea02 · 0xa1ea03 · 0xa1ea04 · 0xa1ea06) |
| 14 | Kairo, Reel (25 s, ten bars of three at 72 BPM) | `14-kairo-reel.mp4` (cover: `14-kairo-reel-cover.jpg`) |

**How it is made.** The rack is two rows of five modules (oscillators,
filter, envelope, sequencer, LFO, VCA, mixer, reverb, scope, output) whose
order is drawn for each edition; their widths are shared out like the HP of a
real case. Cables hang from an output to an input, sagging, each swaying at
its own pace. The music has no drums. A piano, written as a synth (two
strings a little out of tune, partials that die faster as they rise), plays a
waltz: the bass on one, a seventh chord on two, over chords that rock between
the fourth and the first like a gymnopédie. From the third bar a melody
arches over it, slow. Under it, a drone an octave below the bass; around it, a
long room. A few tiny clicks, and once a note stutters, as if the machine
hesitated. In the Reel the sequencer steps on each beat, the scope traces
what sounds, the meter rises with each chord, and the melody is written on
the staff below as it is played.

### 14 · Kairo (Reel)

**EN**

KAIRO, a modular synthesizer that never existed plays a slow waltz on a
piano: a seventh chord on every second beat, a melody that rises and falls,
a deep drone under it, the small clicks of a machine thinking. The melody is
written on the staff as you hear it. Drawn, composed and synthesized by the
same program. Edition 0xa1ea01.

**FR**

KAIRO, un synthétiseur modulaire qui n'a jamais existé joue une valse lente
au piano : un accord de septième à chaque deuxième temps, une mélodie qui
monte et redescend, un bourdon grave dessous, les petits clics d'une machine
qui réfléchit. La mélodie s'écrit sur la portée à mesure qu'on l'entend.
Dessiné, composé et synthétisé par le même programme. Édition 0xa1ea01.

## Seventh room: Parasite

| # | Post | Files |
|---|---|---|
| 15 | Parasite, Reel (24 s, a loop) | `15-parasite-reel.mp4` (cover: `15-parasite-reel-cover.jpg`) |

**How it is made.** Parasite is the first work on a `timeline`. Its scenes
are works of their own, in `parasite/`: *Static* (snow in blocks, new every
frame), *Figure* (someone made of the same blocks in other colours, a
screen on the chest, flickering in and out), *Windows* (a desktop that fills
with windows, one with snow, one a scope, one blinds, one "NO SIGNAL") and
*TestCard* (colour bars, a dial, a countdown). The timeline crosses them with
`glitch` (slices torn sideways, blocks out of place, bursts of static, red
and blue apart), `pixels` and `morph`, and loops: the last glitch leads back
to the snow. The sound is the set's: a drone of pulse waves through a slow
filter, hiss and crackle that rise in the snow, blips, chatter, and, at
every cut, a burst of static (`film.cues`, one event per transition).

### 15 · Parasite (Reel)

**EN**

PARASITE — a broadcast being lost. Snow; someone appears in it, made of the
same blocks; the screen fills with windows; a test card counts down; the
figure comes back and dissolves. Five works on a timeline, cut with glitches,
and a sound made of the same static. The loop never ends. Edition 0xa1ea01.

**FR**

PARASITE — une émission qui se perd. De la neige ; quelqu'un y apparaît,
fait des mêmes blocs ; l'écran se remplit de fenêtres ; une mire compte à
rebours ; la silhouette revient et se dissout. Cinq œuvres sur une timeline,
montées au glitch, et un son fait de la même neige. La boucle ne finit
jamais. Édition 0xa1ea01.

## Eighth room: Requête

| # | Post | Files |
|---|---|---|
| 16 | Requête, Reel (28.5 s, a loop) | `16-requete-reel.mp4` (cover: `16-requete-reel-cover.jpg`) |

**How it is made.** One work per layer, in `requete/`, on a timeline:
*Url* (the address typed into the bar, Enter, the request line), *Dns* (a
query climbs the tree, root, `io`, `bngz`, `alea`, and the answer comes back
as an address), *Handshake* (SYN, SYN-ACK, ACK between two rails, then two
rings mesh and a lock closes: TLS), *Packets* (the request cut into packets,
each taking its own staircase across a grid of routers, the TTL counting
down), *Bits* (the frame as a stream of ones and zeros, the signal they make
on the wire, pulses of light down a fibre) and *Page* (the response: the
page builds itself, block by block, 200 OK). Each is painted in its own
colour; the timeline crosses them with `pixels`, `slide`, `wipe`, `glitch`,
`morph` and a fade through black that leads back to the bar. Every layer has
its sound: keys, blips and an answer, three tones and a chord for the lock,
pulses at every hop, a modem's two-tone song under the bits, and the chord
of the page resolving. The addresses are the ones kept for documentation
(192.0.2.44, 203.0.113.7): nobody's.

### 16 · Requête (Reel)

**EN**

REQUÊTE — the life of one request. An address is typed; the name becomes a
number; two machines shake hands and lock the line; the request is cut into
packets that find their way across the network; on the wire, bits and light;
and back at the top, the page builds itself. One work per layer, on a
timeline, each with its own sound. Edition 0xa1ea01.

**FR**

REQUÊTE — la vie d'une requête. Une adresse est tapée ; le nom devient un
nombre ; deux machines se serrent la main et verrouillent la ligne ; la
requête est découpée en paquets qui trouvent leur chemin à travers le
réseau ; sur le fil, des bits et de la lumière ; et de retour en haut, la
page se construit. Une œuvre par couche, sur une timeline, chacune avec son
propre son. Édition 0xa1ea01.

## Ninth room: Signal

| # | Post | Files |
|---|---|---|
| 17 | Signal, Reel (60 s, a loop) | `17-signal-reel.mp4` (cover: `17-signal-reel-cover.jpg`) |

**How it is made.** One work per stage of a synthesizer voice of the Juno
kind, in `signal/`, and of what comes after, cut fast (scenes of 1.5 to 6.5
seconds, transitions of 0.3 to 0.6, mostly glitches, pixels and morphs; a
few stages come back zoomed or faster): *Dco* (the saw, the pulse with its
width breathing, the sub an octave below), *Noise*, *Lfo* (a triangle a few
times a second, and what it does to the pitch and the pulse width), *Env*
(attack, decay, sustain, release drawn as the line it is, a dot running it
on every beat), *Hpf*, *Vcf* (the harmonics as bars, the low-pass curve
sweeping across them, the resonance a peak at the knee), *Vca* (the wave
shaped by the envelope, then the chorus: the same wave twice, a little late
and apart, one for each ear), *Delay* (a hit and its repeats), *Mix* (the
desk, meters jumping with what plays), *Comp* (the gain-reduction meter
diving on every kick), *Lathe* (the head cutting the groove), *Vinyl* (the
record turning, its grooves wobbling with the beat, the arm), *Needle* (the
stylus in the groove, close up), *Speaker* (the cone, and rings of pressure
leaving it on every kick) and *Room* (the floor).

The track is built the same way, at 128 BPM over thirty-two bars: while the
DCO is on screen, a raw saw + pulse + sub holds A1, straight out; the LFO
makes it waver; the envelope makes notes of it (a bassline drawn for each
edition) and the kick comes in; the filter opens over the bassline and the
hats join; the chorus widens the stabs (a minor ninth, a major ninth a third
below); the full mix brings a lead; the record adds its crackle and a little
wow; a riser lifts the room; and the last beat fades through black into the
oscillator again. Every instrument is a synth written in the work, in the
Juno way: saw + pulse + sub, a resonant low-pass, an envelope, chorus.

### 17 · Signal (Reel)

**EN**

SIGNAL — the path of a sound, from the oscillator to the room. The DCO, the
noise, the LFO, the envelope, the filters, the VCA and the chorus, the delay,
the mix, the compressor, the lathe, the vinyl, the stylus, the speaker, the
floor: one work per stage, twenty-one cuts, and the track is built with them, stage by
stage, Detroit techno at 128. A minute, in a loop. Edition 0xa1ea01.

**FR**

SIGNAL — le chemin d'un son, de l'oscillateur à la salle. Le DCO, le bruit,
le LFO, l'enveloppe, les filtres, le VCA et le chorus, le délai, le mix, le
compresseur, la graveuse, le vinyle, la pointe, l'enceinte, la piste : une
œuvre par étage, vingt et une coupes, et le morceau se construit avec eux, étage par étage,
techno de Detroit à 128. Une minute, en boucle. Édition 0xa1ea01.

## Tenth room: Hana

| # | Post | Files |
|---|---|---|
| 18 | Hana, Reel (60 s, a loop) | `18-hana-reel.mp4` (cover: `18-hana-reel-cover.jpg`) |

**How it is made.** Twelve works in `hana/`, each a stage or a texture of
the flower's life, all built on one idea: the square, and the distance that
draws it (the largest of |dx| and |dy|), so that rings, petals, cells and
seeds are all squares. *Lattice* (the network: a grid of nodes, links there
or not by chance, packets running along them, the grid shaken a little on
every tick), *Seed* (a square turning by steps, square ripples leaving it),
*Roots* (staircase paths from node to node, drawn once and revealed as they
grow), *Stem* (a column of cells swaying by steps, square leaves opening
where it passes), *Bud* (square shells nested and turning their own ways),
*Bloom* (square petals in layers around a blinking heart, opening with an
overshoot), *Macro* (inside a petal: a weave of cells folded fourfold),
*Spread* (seeds leaving the heart along the links to nodes of their own),
*Drift* (seeds on the wind, in rows of their own speed), *Field* (flowers
opening all over the grid), *Wither* (petals falling, the picture coming
apart into larger cells and then grain) and *Data* (a block of bits
changing on every tick, between the stages). Forty-one scenes of 0.5 to 3.3
seconds on a timeline, in a loop; every cut is drawn for each edition: a
glitch most often, pixels or a morph sometimes, now and then a plain jump.
Every scene draws its own grid, links, roots, petals and seeds. The colours
are those of old prints (teal, rust, moss, a faded rose, a VHS blue,
sepia), one set per stage, and the whole film goes through `film` (grain),
`vignette` and `chroma`.

The track follows the six stages at 92 BPM, twenty-three bars, a minute: a
pad in D (Dmaj9, Bm9, Gmaj7♯11, Em9), two saws a little apart under a slow
low-pass, warped by a tape's wow and flutter, and hiss, for the seed; a sub
bass and a soft kick for the roots; a dusty snare, hats and a detuned bell
on the pentatonic, through a dotted-eighth echo, for the stem and the bud;
the full swung beat and an arpeggio for the bloom; the beat broken into
rolls of thirty-second notes for the spread; the drums gone for the fall.
Every kick, ghost note, roll, bell note and arpeggio note is drawn by
chance, and a blip marks each cut.

### 18 · Hana (Reel)

**EN**

HANA 花 — an electronic flower, from seed to seed, in a network. A seed
calls in the grid; roots run along its links; a stem of square cells, a bud
of nested squares, square petals; the seeds leave along the links and on
the wind, flowers open everywhere, the petals fall apart into grain, and a
seed again. Forty-one cuts, every glitch drawn by chance, an IDM track that
grows with it. A minute, in a loop. Edition 0xa1ea01.

**FR**

HANA 花 — une fleur électronique, de graine en graine, dans un réseau. Une
graine appelle dans la grille ; des racines courent le long de ses liens ;
une tige de cellules carrées, un bouton de carrés emboîtés, des pétales
carrés ; les graines partent par les liens et dans le vent, des fleurs
s'ouvrent partout, les pétales tombent en grain, et de nouveau une graine.
Quarante et une coupes, chaque glitch tiré au hasard, un morceau d'IDM qui
pousse avec elle. Une minute, en boucle. Édition 0xa1ea01.

## Eleventh room: Babel

| # | Post | Files |
|---|---|---|
| 19 | Babel, Reel (60 s, a loop) | `19-babel-reel.mp4` (cover: `19-babel-reel-cover.jpg`) |

**How it is made.** After the library that holds every possible book: ten
works in `babel/`. *Page* (forty lines of a script of twenty-five symbols,
each letter a few cells lit in a 3 × 5 grid drawn by chance for the edition,
lines rewritten on the ticks: every page that can be), *Count* (410 pages,
40 lines, 80 symbols, 25 symbols), *Book* (a book in three dimensions,
ray-cast point by point: where the ray from the eye meets the box of its
boards, its spine and its page block, which face, lit from above; a hexagon
on its cover), *Books* (several of them turning at once), *Shelf* (a wall of
five shelves, each spine its own width and height, under a swinging lamp),
*Hexroom* (a gallery from above: six walls, four lined with books, two open
onto the hallways, the shaft and its railing), *Corridor* (hexagonal
doorways one behind the other, in perspective), *Shaft* (down the air shaft:
railings, floors and the spiral stair into the depth), *Mirror* (the gallery
folded sixfold) and *Infinity* (one continuous rise out of a gallery,
through the honeycomb of them, while the camera tips forward until the
floor of galleries runs to a lit horizon, floors below coming into view).
Forty-one scenes, the last five pieces of that one rise; every cut drawn for
each edition. The colours of candlelight, bone, verdigris and night, an
oxblood for the numbers, through `film`, a heavy `vignette` and `chroma`.

The track, at 76 BPM over nineteen bars: a drone on D and the rustle of
pages turned for the script; a choir (saws through the formants of an "ah")
in D Phrygian dominant (D, E♭, D7, Cm) and a bell out of tune with itself for
the book; a broken half-time beat for the shelves and the gallery; rolls of
thirty-second notes and stutters for the hallways and the shaft; for the rise,
the kick on every beat, a swell before each stage and a last bell. Every
page turned, every click, roll and bell is drawn by chance.

### 19 · Babel (Reel)

**EN**

BABEL — the library that holds every book. A script of twenty-five symbols,
rewritten on every tick; a book turning in the dark; the shelves, a
gallery of six walls, the hallways, the shaft, the mirror; then one long
rise out of a gallery, through the honeycomb of them, until the floor runs
to the horizon. Forty-one cuts, a dark choir, a broken beat. A minute, in a
loop. Edition 0xa1ea01.

**FR**

BABEL — la bibliothèque qui contient tous les livres. Une écriture de
vingt-cinq symboles, réécrite à chaque battement ; un livre qui tourne dans
le noir ; les rayons, une galerie à six murs, les couloirs, le puits, le
miroir ; puis une longue montée hors d'une galerie, à travers leur alvéole,
jusqu'à ce que le sol rejoigne l'horizon. Quarante et une coupes, un chœur
sombre, un rythme brisé. Une minute, en boucle. Édition 0xa1ea01.

### Hashtags

```
#generativeart #creativecoding #algorithmicart #proceduralart #codeart
#genart #newmediaart #digitalart #topography #nocturne #stainedglass #rustlang
#paris #metro #transitmap #cartography
#reactiondiffusion #gameoflife #emergence #biology #petridish
#lofi #lofihouse #sunset #vinyl #detroittechno #acid #techno #drummachine #synthesizer #japanesedesign
#babel #library #infinite #hexagon #borges #3d #raycasting #choir #darkambient
#flower #squares #network #growth #boardsofcanada #idm #tape #analog
#synthesizer #juno #modular #signalpath #vinyl #http #dns #tcp #osi #networking #glitch #glitchart #vhs #nosignal #idm #piano #modularsynth #eurorack #detroittechno #acid #techno #drummachine #synthesizer #japanesedesign
```
