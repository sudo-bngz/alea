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
| **Kairo** (回路) | `kairo.alea` (+ `fonts/`) | After Kikai, a modular synthesizer that never existed: two rows of modules racked in another order each edition, patched with hanging cables. Also a Reel without drums: a slow piano waltz over a deep drone, the melody written on a staff as it plays. |

## Replaying the editions

```sh
alea isohypses.alea --seed 0xa1ea03          # writes the PNG of that edition
alea nocturne.alea  --seed 0xa1ea06          # the image, and the 1080×1920 Reel (needs ffmpeg)
alea exhibition.alea --seed 0xa1ea43         # the opening card
alea correspondances.alea --seed 0xa1ea00    # the plan, and the 1080×1920 Reel of its day
alea culture.alea --seed 0xc06               # the plate, and the Reel of its growth (2 to 3 minutes of growing first)
alea kairo.alea --seed 0xa1ea01              # the rack, and the 1080×1920 Reel of its waltz (about 30 minutes)
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

### Hashtags

```
#generativeart #creativecoding #algorithmicart #proceduralart #codeart
#genart #newmediaart #digitalart #topography #nocturne #stainedglass #rustlang
#paris #metro #transitmap #cartography
#reactiondiffusion #gameoflife #emergence #biology #petridish
#lofi #lofihouse #sunset #vinyl #detroittechno #acid #techno #drummachine #synthesizer #japanesedesign
#idm #piano #modularsynth #eurorack #detroittechno #acid #techno #drummachine #synthesizer #japanesedesign
```
