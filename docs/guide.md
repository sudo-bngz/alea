# The alea guide

This guide teaches alea from the first line to works made of works. Each
chapter goes with one file of the tour in `examples/tour/`: read the chapter,
run the file, change it, run it again.

```sh
alea examples/tour/01-hello.alea            # writes the edition's files in the current folder
```

Or open the playground (`web/dist/playground.html`) and pick the tour in the
example menu. For the exact rules (every parameter, default and limit) see the
[reference](reference.md).

---

## 1. A work, and its editions

> Tour: `01-hello.alea`

```alea
work Hello {
  field waves = sin(x * ~between(5, 40))
  render image(waves)
}
```

A program in alea is a **work**. A work doesn't describe one picture; it
describes a whole family of possible pictures. Each time you run it, alea
draws one member of that family, called an **edition**, and renders it.

`~between(5, 40)` is the only uncertain part of this work: each run draws a
number between 5 and 40, so each edition has a different number of stripes.
Everything else is fixed.

Every edition has a **seed**, printed when it's made:

```
`Hello` — edition 0x6c76b826fc738291
```

The seed is the edition's identity. Give it back and you get exactly the same
files, byte for byte, on any computer and in the browser:

```sh
alea examples/tour/01-hello.alea --seed 0x6c76b826fc738291
```

So a work is like a printing plate and a seed like a print number, except
that every print is different, and any print can be made again.

**The shape of a file.** A file holds one work: `work`, a name, and between
braces a list of statements. Comments start with `//` (or `#`) and run to
the end of the line.

---

## 2. Shapes, and what alea decides

> Tour: `02-shapes.alea`

```alea
let anywhere = circle()
let placed   = square(x: 0.72, y: 0.28, size: 0.18)
let exact    = ring(x: 0.5, y: 0.5, size: 0.85, width: 0.01)
```

alea has a vocabulary of **primitives**: shapes (`circle`, `ring`, `rect`,
`square`, `line`, `blob`), textures, transforms and more. The rule that
matters most:

**Anything you leave out, alea decides.**

`circle()` is a circle somewhere, of some size: alea draws both. Give it `x`
and `y` and the position is yours, the size still alea's. Give everything and
nothing is left to chance. Precision is how you take control back.

**Where things are.** The canvas goes from 0 to 1 in both directions, with
`(0, 0)` at the top-left and `(1, 1)` at the bottom-right. Sizes are fractions
of the canvas: `size: 0.5` is half its width. Angles are in degrees.

**Arguments.** Name them (`circle(x: 0.5, size: 0.2)`, in any order) or give
them in the order the reference lists (`circle(0.5, 0.5, 0.2)` is x, y,
size). Unnamed ones come first.

**Naming things.** `let name = …` gives a name to a value so you can use it
again, in the lines that follow. Statements run from top to bottom, so a name
must be given before it's used; giving the same name again replaces it from
that line on.

---

## 3. Chance

> Tour: `03-chance.alea`

alea has three ways to involve chance. The first is the **draw**.

`between(0.08, 0.3)` is a **distribution**: a value that hasn't been decided
yet. On its own it isn't a number; you can name it, but you can't compute
with it. `~` draws from it, which gives a number.

```alea
let spread = between(0.08, 0.3)      // a distribution
let r      = ~spread                 // a number, drawn once
let left   = circle(size: r)
let right  = circle(size: r)         // same size as `left`
let a      = circle(size: ~spread)
let b      = circle(size: ~spread)   // a different size from `a`
```

**When is a draw made?** Where the `~` is written, once:

- in a `let`, once per edition. The name keeps that value everywhere it's used.
- written twice, twice: two independent values.
- inside a pattern's events (chapter 10), once per event.
- inside a work's parameter default (chapter 15), once per call.
- inside a `choose` branch (chapter 4), only if that branch is chosen.

**The distributions.**

| Distribution | Gives |
|---|---|
| `uniform(a, b)`, also `between(a, b)` | any value from a to b, all equally likely |
| `normal(m, d)`, also `around(m, d)` | values around m; about two in three fall within d of it |
| `beta(a, b)` | a value from 0 to 1, leaning towards 0 when a < b, towards 1 when a > b |
| `poisson(λ)` | a whole number, λ on average (good for "how many") |
| `exponential(λ)` | mostly small values, occasionally large ones (average 1/λ) |

The second kind of chance is the **grain**, `~~`. Where `~` draws once,
`~~` draws again at every point of the canvas (and every moment of a video):

```alea
field grain = ~~uniform(0, 1) * 0.12
```

A grain is still fixed by the seed: the same edition has the same grain.

The third is the **choice**, next.

---

## 4. Choices

> Tour: `04-choices.alea`

`choose` picks one of several branches:

```alea
let motif = choose {
    3: circle(x: 0.5, y: 0.5, size: 0.55)
  | 2: square(x: 0.5, y: 0.5, size: 0.5, angle: 45)
  | 1: blob(x: 0.5, y: 0.5, size: 0.6)
}
```

Branches are separated by `|`. The number before the colon is a **weight**:
here the circle comes up 3 times in 6, the square 2 in 6, the blob 1 in 6.
Without weights, every branch is equally likely.

A choice is **structural**: it doesn't nudge a number, it changes what the
work is made of. Only the chosen branch is drawn at all.

A branch can give anything: a shape, a number (`choose { 2 | 3 | 4 }`), or a
text (`choose { "ink" | "sea" }`, handy for palettes).

---

## 5. Combining

> Tour: `05-combining.alea`

Shapes are 1 inside and 0 outside, with a soft edge in between. Combining
them is combining those values:

| Write | You get |
|---|---|
| `merge(a, b, …)` | everything, on top of each other |
| `overlap(a, b, …)` | only where they all are |
| `cut(a, b)` | a, with b removed |
| `invert(a)` | inside and outside swapped |
| `a + b` | the values added: layers |
| `a * 0.4` | a lighter tone of a |
| `mix(a, b, amount)` | a blend from a (0) to b (1) |
| `threshold(a, level: 0.5)` | soft tones turned into a crisp edge |

**Several renders.** This file has seven `render image` lines, and running
it writes seven pictures. A work can render as many times, and in as many
media, as you like. All its renders belong to the same edition.

---

## 6. Transforms

> Tour: `06-transforms.alea`

A transform takes a shape (or a texture, or a picture, or a whole
composition) and changes where it's drawn:

| Transform | Effect |
|---|---|
| `move(s, x: 0.1, y: 0)` | shifts by a distance |
| `rotate(s, angle: 30)` | turns around the centre (or around `x`, `y`) |
| `zoom(s, by: 2)` | scales; 2 is twice as big, 0.5 half |
| `tile(s, count: 3)` | repeats the whole canvas in a 3 × 3 grid |
| `mirror(s, axis: "both")` | adds the reflection across the centre line(s) |
| `kaleidoscope(s, sides: 6)` | repeats what lies to the right of the centre, all around it |
| `warp(s, amount: 0.05)` | bends everything, like a hand-drawn line |

Transforms nest, and read from the inside out:

```alea
rotate(kaleidoscope(warp(leaf, amount: 0.05), sides: 5), angle: 18)
```

warps the leaf, folds it into a five-petal flower, then turns the flower.

Like every primitive, a transform decides what you leave out:
`rotate(shape)` picks an angle; `kaleidoscope(shape)` picks how many sides.

**Watch out:** this includes the parameter you didn't think of.
`move(shape, x: 0.01)` moves right by 0.01 *and* up or down by a random
amount, because `y` was left to alea. When you need an exact transform (for
example to compare a field with a slightly shifted copy of itself), give every
parameter: `move(shape, x: 0.01, y: 0)`.

---

## 7. Textures

> Tour: `07-textures.alea`

Textures fill the whole canvas with values from 0 to 1: `stripes`, `waves`,
`ripples`, `dots`, `checker`, `gradient`, `glow`, `clouds`, `grain`, and
`scatter` (random specks). Most have a `softness` from 0 (sharp) to 1
(smooth). They combine with shapes like anything else: stripes seen
through a circle is `overlap(stripes(), circle())`.

---

## 8. Words and pictures

> Tour: `08-words-and-pictures.alea`

```alea
let word  = text("alea", x: 0.5, y: 0.5, size: 0.3, font: "bold")
let photo = picture("../assets/astronaut.png")
```

`text` draws a line of text as a shape; `size` is the height of the line.
Fonts: `"sans"` (default), `"bold"`, `"serif"`, `"mono"`, or the path to any
`.ttf` / `.otf` file.

`picture` brings in a PNG, JPEG or WebP image, as a field of its brightness.
By default it fills the canvas width, centred. `channel: "alpha"` uses a
transparent PNG's transparency instead, which turns a logo or a cut-out into
a shape. Paths are relative to the `.alea` file.

These two are the exception to "alea decides": they show *your* content, so
by default text is level and a picture fills the width.

Both are shapes: `warp(text(…))`, `kaleidoscope(picture(…))`,
`overlap(photo, text("SPACE"))` (the photo seen through the letters).

---

## 9. Fields: the maths underneath

> Tour: `09-fields.alea`

Underneath every shape is a **field**: a number at every point `(x, y)` of
the canvas, and every moment `t`. `x` and `y` are themselves fields:

```alea
field across = x                                   // 0 on the left, 1 on the right
field centre = distance(x, y, 0.5, 0.5)            // 0 in the middle
field rings  = sin(centre * 60)
field marble = sin(x * 12 + 4 * noise(x * 4, y * 4, 0))
```

Arithmetic (`+ - * / % ^`) and functions (`sin`, `abs`, `noise`,
`smoothstep`…) build new fields from old ones. A comparison gives 1 where
it's true and 0 where it's false, so `(x > 0.5) * rings` is rings on the
right half only.

`field name = …` is like `let`, but insists the value is a field (not a text
or a distribution), which catches mistakes early.

You never *have* to write maths: every primitive is a field built this way.
But the two mix freely, so `circle() * marble` is a marbled disc.

---

## 10. Patterns: many of something

> Tour: `10-patterns.alea`

```alea
pattern rain = repeat 300 event(
    x: ~uniform(0, 1),
    y: ~beta(5, 2),
    intensity: ~between(0.3, 1),
    size: ~between(0.002, 0.01)
)
```

A **pattern** is a set of **events**. `repeat N event(…)` makes N of them,
and every `~` inside is drawn again for each event, so the 300 drops differ.
An event has five attributes; what you leave out is drawn at random:

| Attribute | Meaning | If omitted |
|---|---|---|
| `x`, `y` | where | anywhere |
| `t` | when (0 to 1) | any time |
| `intensity` | how strong | 1 |
| `size` | how big (the spread of its spot) | 0.02 |

Inside an event, `i` is the event's number (0, 1, 2, …) and `n` is how many
there are, so `i / n` goes from 0 to almost 1: use them for order, spirals
and sequences.

A pattern used as a field shows each event as a soft spot. In a sound, its
events become notes (chapter 13). `scatter(count, size)` is a ready-made
pattern of random specks.

---

## 11. Constraints: what must be true

> Tour: `11-constraints.alea`

```alea
constraint maximum(overlap(a, b)) < 0.5      // the circles never touch
constraint count(stars) >= 35 and count(stars) <= 45
```

A **constraint** is a condition every edition must meet. When one fails, alea
throws the whole edition away and draws it again (up to 1000 times). You state
the goal; chance finds a way to reach it.

To write conditions about whole fields, alea measures them over the canvas:
`mean`, `stddev` (how varied), `minimum`, `maximum`. `count(pattern)` gives
the number of events. Conditions combine with `and` and `or`.

If no edition can satisfy the constraints, alea says which one kept failing.
When an edition needed several tries, it says so:

```
`Constraints` — edition 0x000000000000002a — 3 attempts to satisfy the constraints
```

---

## 12. Time: works that move

> Tour: `12-time-and-video.alea`

`t` is time. In an image it's 0 (or the render's `at`); over a video or a
sound, it goes from 0 at the start to 1 at the end.

```alea
let beat  = lfo()                             // 0 → 1 → 0 over the video
let heart = circle(x: 0.5, y: 0.5, size: 0.15 + 0.2 * beat)
let spin  = rotate(stripes(), angle: t * 180)
```

Any parameter can use `t`. `lfo()` goes from 0 up to 1 and back to where it
started, so motion built on it loops without a jump; `lfo(3)` does it three
times. It is `0.5 - 0.5 * cos(t * tau)` (`tau` is a full turn, 2π), which
you can still write yourself. Chapter 21 has more ways to move.

`render video(piece, duration: 4)` writes an MP4 (with a soundtrack: see
the next chapter) using ffmpeg; `format: "gif"` writes a silent loop.
`render image(piece, at: 0.5)` takes a still from the middle.

---

## 13. Sound: layers of listening

> Tour: `13-sound.alea`

A soundtrack is made of **layers**, each listening to the work in its own way:

```alea
render sound(drone(sky, instrument: "strings", scale: "dorian", key: 110),
             notes(bells, instrument: "marimba", scale: "dorian", key: 220, tempo: 100),
             texture(mist, kind: "wind"),
             duration: 12, room: 0.5, echo: 0.2)
```

- **`drone(field)`**: voices that hold notes of a scale. Each listens to the
  field along one row of the canvas (the top row sounds lowest): loud where
  the field is high, brighter when louder, and on the left or the right
  where the field is higher on that side.
- **`notes(pattern)`**: each event plays a note. Its `t` is when (on the
  beat if you give a `tempo`), its `y` which note of the scale (higher on the
  canvas, higher in pitch), its `x` where it sounds, its `size` how long it
  rings, its `intensity` how loud.
- **`texture(field)`**: an ambience (`"wind"`, `"waves"`, `"rain"`,
  `"crackle"`) that grows louder as more of the field lights up.

Each layer has an `instrument` (strings, choir, organ, glass…; for notes:
bell, pluck, marimba, kalimba, drum…), a `scale` (major, minor, dorian,
pentatonic, blues…), a `key` in hertz and a `gain`. Around the mix, `room`
gives a space (0 none, 0.9 a cathedral) and `echo` a repeat between left and
right.

A layer can listen to anything, not only what's on screen: the drone of a
video can follow the sky while the picture shows the sea. In a video, give
the layers to `sound:`:

```alea
render video(piece, duration: 20, sound: [drone(sky), notes(rain)], room: 0.6)
```

Without layers, `render sound(field, pattern)` makes the original sound: a
sine drone and bell notes, in mono.

---

## 14. Sculpture

> Tour: `14-sculpture.alea`

`render sculpture(field)` turns values into heights: low values are the
floor, high values the peaks, on a solid base. The result is an STL file,
the format 3D-printing software reads, measured in millimetres. `side` is
the width, `base` the thickness of the base, `relief` the height of the
tallest point as a fraction of the side, and `resolution` how fine the
surface is.

---

## 15. Works made of works

> Tour: `15-components.alea` and `lib/flower.alea`

A work can be used inside another. Give it parameters and an `output`:

```alea
work Flower(cx = ~between(0.2, 0.8), cy = ~between(0.2, 0.8),
            size = ~between(0.12, 0.22), petals = choose { 5 | 6 | 8 }) {
  let petal = blob(x: cx + size * 0.55, y: cy, size: size * 0.7)
  output kaleidoscope(petal, sides: petals, x: cx, y: cy)
  render image(output)          // only when this file runs on its own
}
```

and bring it in with `use`:

```alea
use "lib/flower.alea"

work Garden {
  let a = Flower()                                   // a new draw
  let b = Flower()                                   // another one
  let centre = Flower(cx: 0.5, cy: 0.5, petals: 12)  // some decisions taken back
  render image(merge(a, b, centre))
}
```

The rules:

- **Every call is a new draw.** `Flower()` twice is two different flowers. To
  use one flower twice, bind it: `let f = Flower()`.
- **Parameters work like a primitive's**: a caller gives some, the defaults
  draw the rest. A parameter without a default must be given
  (`work Frame(art, margin = 0.08)`), and a parameter can be a shape, which
  makes a work a reusable effect.
- **One seed still replays everything**, components included. Each instance
  keeps its own grain and noise.
- **Constraints stay local**: a component that fails its own constraint is
  redrawn on its own, without redrawing the piece around it.
- **A component's `render`s are ignored** when it's used; only `output`
  counts. The same file is a standalone piece and a component.
- `x`, `y`, `t` can't be parameter names, since they mean the canvas and
  time: call a centre `cx`, `cy`.

---

## 16. Everything together

> Tour: `16-lighthouse.alea`

The last file draws a horizon at a random height, a sky of clouds and a
number of stars that a constraint keeps above 110, a sea of warped waves, and
a lighthouse whose beam turns once over the video. It renders an image, a
video and a sound from the same edition.

---

## 17. Growing: fields with a memory

> Tour: `17-life.alea`

Everything so far is a function of the place and the moment: alea can compute
any point of any frame on its own. Some things can't be described that way.
In Conway's Game of Life, whether a cell is alive depends on its neighbours
one generation earlier, and theirs on the generation before. For that, a
field has to remember. That is what `grow` does:

```alea
grow over 160 for 360 {
  cells = ~~uniform(0, 1) < 0.3
    then neighbours(cells) == 3 or (cells and neighbours(cells) == 2)
}
```

- `over 160`: the canvas is a grid of 160 × 160 cells. `for 360`: there are
  360 generations after the first.
- Each line reads **`name = start then next`**. `start` is generation 0.
  `next` computes the following generation; inside it, `cells` means the
  previous generation.
- Inside `next`, three helpers look at the cells around each cell:
  - `neighbours(f)`: the sum of the 8 cells around it;
  - `spread(f)`: how much the surroundings exceed the cell itself (a
    diffusion: add it, and the field spreads out evenly);
  - `nearby(f, r)`: the average over a disc of `r` cells.
- After the block, `cells` is an ordinary field. `t` runs through the
  generations, from the first (`t` = 0) to the last (`t` = 1), and between
  cells and between generations the values are blended smoothly. The grid
  wraps around: what leaves on the right comes back on the left.

Several lines in one block **grow together**: each `next` sees the previous
generation of all of them. That's how two chemicals react and spread (a
reaction–diffusion, the maths behind spots, stripes and corals):

```alea
grow over 256 for 6000 {
  food = 1 - 0.5 * seeds
    then food + spread(food) - food * life * life + 0.055 * (1 - food)
  life = 0.25 * seeds
    then life + 0.5 * spread(life) + food * life * life - 0.117 * life
}
```

Chance still works the usual way. `~` in a rule is drawn once, `~~` gives
each cell its own draw (and a new one at each generation). Inside `next`,
`t` is the generation's moment, so a rule can change over time.

Growing takes time: it is computed when the edition is drawn, cell by cell,
generation by generation, before anything is rendered. A grid of 128 cells
for a few hundred generations is instant; 256 cells for thousands of
generations takes a minute or two.

---

## 18. Lists and functions

> Tour: `18-lists.alea`

**A list** holds several values: `[0.2, 0.5, 0.8]`. A **point** is a list of
two numbers, `[x, y]`, and `.x` and `.y` read them. `list[0]` is the first
element, `list[-1]` the last; `count(list)` says how many there are.

```alea
let stops = [[0.1, 0.8], [0.4, 0.3], [0.9, 0.5]]
let first = stops[0]
let tip = circle(first.x, first.y, 0.05)
```

**A loop** builds a list, one element for each element of another. `range(n)`
counts from 0 to n − 1:

```alea
let dots  = [circle(p.x, p.y, 0.03) for p in stops]     // one circle per stop
let bars  = [rect(0.1 + k * 0.1, 0.5, 0.05, 0.6) for k in range(8)]
let grid  = [[a / 4, b / 4] for a in range(5) for b in range(5)]   // two loops: 25 points
```

Each `~` in a loop is drawn again for every element:
`[~between(0, 1) for k in range(10)]` is ten different numbers.

**Putting a list together.** `merge(list)` draws everything in it,
`overlap(list)` keeps only where all of it is. `sum`, `mean`, `minimum`,
`maximum` work on lists of numbers or fields. `join(a, b)` puts lists end to
end, `slice(list, 1, -1)` takes a part, `reverse(list)` turns it round.

**Arithmetic works element by element**, so points add, scale and blend like
numbers: `p + [0.1, 0]` moves a point, `(a + b) * 0.5` is halfway between two,
and so is `mix(a, b, 0.5)`. `distance(a, b)` is the distance between two
points.

**Paths.** A list of points is also a line to draw and a road to follow:

| | |
|---|---|
| `path(points, width, softness)` | the line through all the points, with round joints |
| `length(points)` | how long that line is |
| `along(points, d)` | the point at distance `d` from the start (`t * length(points)` travels it once over the video) |

**A function** names a calculation you use more than once:

```alea
let snap(v) = floor(v * 24 + 0.5) / 24
let star(x0, y0) = [x0 + ~around(0, 0.04), y0 + ~around(0, 0.04)]
let a = star(0.3, 0.4)      // a new draw at every call
```

A function's body is one expression. It sees the names given before it (not
the ones after, and not itself), and its parameters can have defaults:
`let shade(v, k = 0.5) = v * k`.

**Values in texts.** Inside a text, `{…}` puts a value: `"{h} h"`, and
`"{h:02}:00"` pads it with zeros to two digits, which writes `08:00`.

---

## 19. Paintings: mosaics and overlays

> Tour: `19-paintings.alea`

A field is numbers; a render turns them into colours with one palette, over
the range the field takes. Put two works in one field and they share that
palette and that range. To keep each work as itself, **paint** it:

```alea
let map  = paint(Isohypses(), palette: "ink")
let city = paint(Correspondances(), palette: "sea")
```

A painting is a field in colour, with its own palette and its own range
(`range: [0, 1]` fixes it instead of measuring it), and transparent where its
`alpha` field is 0. Paintings are laid out and stacked:

- **`mosaic(paintings, columns, gap, background)`**: a wall of screens.
  Each cell shows a whole painting (its unit square, centred), side by side,
  each running at the same moment in a video.
- **`place(painting, x, y, width, height)`**: one painting anywhere on the
  canvas, at any size: a picture in a picture.
- **`overlay(below, above, …, blend, opacity, mask)`**: one painting over
  another. `blend` is `"normal"`, `"add"`, `"screen"`, `"multiply"`,
  `"lighten"`, `"darken"`, `"difference"` or `"overlay"`; `opacity` how much
  shows; `mask` a field saying where. To give layers different blends, put an
  overlay inside another.

`render image` and `render video` show a painting in its own colours (so
they take no `palette`). A work can give a painting as its `output`, and then
keeps its colours wherever it is used. A painting can't go into arithmetic
any more: it is the end of the line, made to be seen.

```alea
let wall = mosaic([paint(Isohypses(), palette: "ink"), paint(Rosace(), palette: "ember")], columns: 2)
render video(overlay(wall, place(paint(Culture(), palette: "night"), x: 0.8, y: 0.8, width: 0.3)), duration: 15)
```

---

## 20. Colours of your own

> Tour: `20-colours.alea`

A field stays a number everywhere; colour is decided at the end, by the
palette. Besides the five built in, a palette can be **a list of colours**:

```alea
let mondrian = ["#f2efe6", "#f5d000", "#d40920", "#1356a2", "#111111"]
render image(city, palette: mondrian)                          // a gradient through them
render image(city, palette: mondrian, bands: 5, range: [0, 1]) // five flat colours
```

- A plain list is a **gradient**, spread evenly from the lowest value to the
  highest.
- **`bands: n`** cuts it into n flat levels. With as many bands as colours,
  each colour gets an equal share of the values and nothing is blended: a
  field that takes the values 0, 0.25, 0.5, 0.75 and 1 shows exactly five
  colours. It works on named palettes too (`palette: "ember", bands: 6`
  posterizes).
- **`range: [0, 1]`** maps the values as they are, without stretching them
  to the range they take, so 0.25 is always the second colour.
- **Placed colours**, `[[0, "#fff"], [0.6, "#fff"], [0.6, "#d40920"], [1, "#111"]]`,
  sit at chosen values; two at the same value make a sharp edge (white up to
  0.6, then red).

The same options work on `paint(…)`, so each painting in a mosaic can have
colours of its own.

---

## 21. Animation: when things happen

> Tour: `21-animation.alea`

`t` runs from 0 to 1 over a video, and anything can follow it. But writing
"from 0.2 to 0.35, grow, overshooting a little" by hand is arithmetic. These
functions say it directly:

- **`progress(start, end)`**: 0 before `start`, 1 after `end`, a straight
  line between. The building block: `progress(0.2, 0.35)` is a little clock
  that runs only during that stretch.
- **`ease(v, kind)`**: the same journey from 0 to 1, with a feel: `"linear"`,
  `"in"` (starts slowly), `"out"` (arrives slowly), `"in-out"` (the default),
  `"sine"`, `"back"` (overshoots, then settles), `"elastic"` (wobbles) or
  `"bounce"` (lands, bounces, lands).
- **`keys([[time, value], …], ease)`**: a value that passes through key
  frames. The values can be numbers, fields or points:
  `keys([[0, [0.2, 0.7]], [0.5, [0.5, 0.2]], [1, [0.2, 0.7]]], ease: "in-out")`
  is a point that travels and comes back.
- **`sequence([a, b, c], fade)`**: one thing after another, each holding an
  equal share of the time, crossfading over `fade`. Fields or paintings.
- **`at(work, time)`**: a field or a painting at another moment.
  `at(Culture(), 0.3)` is frozen, `at(Culture(), 1 - t)` runs backwards, and
  `at(Culture(), ease(t, "out"))` slows down to its end. Inside the work,
  time is whatever you give it, all the way down.

```alea
let pop  = ease(progress(0.2, 0.35), "back")
let halo = ring(0.5, 0.5, 0.5 * pop, 0.015, 0.01)
let ball = circle(0.12, 0.1 + 0.8 * ease(fract(t * 4), "bounce"), 0.05)
render video(sequence([paint(calm), paint(storm), paint(still)], fade: 0.15), duration: 12)
```

All of these take `time:` to follow another clock than `t`: `fract(t * 4)`
repeats four times, `progress(…, time: other)` follows anything.

**Going round.** Most motion repeats, and two helpers say so:

- **`lfo(times, shape, phase, from, to)`**: a value that cycles, the
  "low-frequency oscillator" of synthesizers. `lfo()` goes from 0 to 1 and
  back once over the work; `lfo(4)` four times (keep it a whole number and
  the video loops). `shape` is `"sine"` (the default), `"triangle"`,
  `"saw"` (a ramp), `"square"` or `"pulse"` (a hit at the start of each
  turn, fading). `from` and `to` set the range, and `phase` shifts it, in
  turns: `[lfo(phase: k / 9) for k in range(9)]` is nine values each a ninth
  out of step. `cc(lfo(8), 74)` sweeps a synth's filter.
- **`orbit(radius, times, phase, x, y)`**: a point `[x, y]` going round
  (0.5, 0.5) by default. `times: [2, 3]` turns twice across and three times
  down: a Lissajous figure. `radius: [0.4, 0.2]` makes an ellipse.

```alea
let p     = orbit(0.3, times: [2, 3])
let comet = circle(p.x, p.y, 0.04)
let ring  = [orbit(0.28, phase: k / 9) for k in range(9)]   // nine points, evenly round
let flash = lfo(16, shape: "pulse")                        // sixteen hits
```

---

## 22. Filters

> Tour: `22-filters.alea`

A filter changes how a picture is seen, not what it is. There are two
moments to apply one.

**On a field**, before colour, so the result is still a field you can
compute with:

- `blur(f, 0.01)`: softened over that distance (a fraction of the canvas).
- `pixelate(f, 0.02)`: in square cells of that size.
- `edges(f)`: only where the field changes, the outlines of shapes.
- `relief(f, angle: 135, depth: 10)`: lit from one side (the angle the light
  comes from, 90 above, 135 the upper left), like an engraving.
- `posterize(f, 6)`: in six flat levels.
- `film(f, 0.08)`: grain, new at every frame.
- `vignette(f, 0.5)`: darker towards the corners.

**On a painting**, after colour, as the very last step: `blur`, `pixelate`,
`film` and `vignette` work on its pixels too, and two more only make sense
in colour:

- `bloom(p, 0.8)`: light bleeding out of the bright parts.
- `chroma(p, 0.5)`: red and blue drifting apart towards the edges, like a
  cheap lens.

```alea
let night = paint(max(lights, land * 0.3), palette: "ember")
render image(vignette(film(chroma(bloom(night, 2.5), 0.8), 0.06), 0.6))
```

A field blur samples the field 40 times around each point, so it costs 40
times the field; a painting blur works on pixels and costs the same whatever
is underneath. For a large blur on a heavy work, paint it first.

---

## 23. Input: works in a pipe

> Tour: `23-input.alea`

A work can read what you give it, and give back what it makes, so works
chain like any command:

```sh
cat portrait.png | alea contours.alea > map.png
cat photo.jpg | alea soften.alea | alea poster.alea > print.png
alea contours.alea --in portrait.png -o .      # the same with files
```

In the work, **`input()`** is what came in: through the pipe, or with
`--in file`. Give it a default, so the work also runs on its own:

```alea
let photo = input(or: "portrait.png")    // a file next to the work
let photo = input(or: clouds())          // or any value
```

An image becomes fields, which is how everything in alea is made:

- **`photo.luma`**, and `.red`, `.green`, `.blue`, `.alpha`, `.hue`,
  `.saturation`, `.lightness`: one channel each, from 0 to 1. The image
  covers the canvas, and the canvas takes the image's size (or its
  proportions, if you give a width), so what comes out lines up with what
  went in.
- **`photo.painting`**: the image in its own colours, for `overlay`,
  `mosaic` and the filters.
- **`colours(photo, 6)`**: its six main colours, darkest first. That's a
  palette, so a work can draw something new in someone else's colours.
- `photo.width`, `photo.height`: its size in pixels.

Everything you know applies. `blur(photo.luma, 0.01)` softens it (a still
picture is blurred once, on its pixels, so it costs nothing per frame),
`edges` finds its outlines, `posterize` cuts it into levels, `relief`
engraves it, `warp` and `kaleidoscope` bend it, and `t` animates it.

**Piped out.** When the output of alea isn't a terminal and no `-o` is
given, the work's first render goes to it (and the edition's line to the
error output). `-o -` asks for it explicitly.

**Which edition.** An edition is now its seed *and* its input: the line
alea prints shows both (`edition 0x… · input 9ea63049`, a fingerprint of
the input). `--seed-from-input` makes the input decide the seed, so the
same picture always gives the same edition.

Inputs can also have names, for works that take several:
`alea duet.alea --in face=me.png --in voice=me.wav`, read with
`input("face")`. A text input gives `.text`, `.words`, `.lines`, and
`.rows` (each line split on commas or spaces, numbers as numbers): data to
draw with.

---

## 24. Listening: a sound as input

> Tour: `24-listening.alea`

A sound (a WAV file; anything else through ffmpeg) becomes time:

```sh
ffmpeg -i song.mp3 -f wav - | alea equaliser.alea -o .
```

- **`song.level`**: how loud, from 0 to 1, along the sound. It's a field of
  `t`, like `sin(t * tau)`, so it goes anywhere a number goes:
  `circle(0.5, 0.5, 0.1 + 0.3 * song.level)`.
- **`song.bass`**, **`song.mid`**, **`song.treble`**, and
  **`band(song, 200, 800)`** for any band of frequencies (in Hz).
- **`song.pulse`**: 1 on each beat, fading within a fifth of a second. The
  simplest way to make something hit with the music.
- **`song.beats`**: the beats as a pattern, one event per beat with its
  strength as intensity. Patterns can be drawn (a long exposure of every
  beat at once) and played: `notes(song.beats, instrument: "glass")` echoes
  the rhythm.
- `song.duration`, in seconds.

A video that reads a sound lasts as long as the sound, and
**`sound: [song]`** puts the sound itself in the soundtrack, as it is,
alongside drones and notes. In other words: a music video, a different one
for every seed.

---

## 25. MIDI: the music, for other instruments

> Tour: `25-midi.alea`

The layers of a soundtrack already decide notes: a `notes` layer strikes a
note of the scale for each event, a `drone` holds notes that swell and fade.
**`render midi`** writes those decisions as a MIDI file, for a DAW, a
synthesizer, a score editor:

```alea
let music = [drone(tide, voices: 4, scale: "dorian"), notes(melody, instrument: "marimba", tempo: 96),
             cc(0.5 + 0.5 * sin(t * tau), 74)]
render sound(music, duration: 20)              // what alea plays
render midi(music, duration: 20, tempo: 96)    // the same music, note for note
```

- Each layer is a **track** on its own **channel**, with a General MIDI
  instrument (strings, marimba, choir…), so the file sounds right anywhere.
- `notes`: one note per event, at the same moment and pitch as in the
  sound, its strength as **velocity**. Drums go to channel 10: low on the
  canvas a kick, then snare, hats, cymbals at the top.
- `drone`: each voice a held note, struck when it rises and released when it
  fades, and the loudness of the layer as **expression** (controller 11).
- **`cc(field, number)`** turns a field (from 0 to 1) into a **controller**:
  a knob that moves on its own. 74 is often a synth's filter, 1 its
  modulation wheel; `channel:` (1–16) says which instrument hears it. A
  `cc` is silent in alea's own sound.
- The tempo (`tempo:`, or the first `notes` layer's) sets the file's grid,
  so the bars line up in a DAW.

**Live.** The same music can be played as it is computed, to any MIDI
instrument:

```sh
alea modes.alea --midi                 # on a new MIDI port called "alea"
alea modes.alea --midi-port "Minilogue" # to an existing one (alea --midi-ports lists them)
alea modes.alea --midi --loop          # a new edition every time one ends, until Ctrl-C
alea modes.alea --midi --sync          # following a DAW's MIDI clock: its tempo, start and stop
alea modes.alea --midi-port print      # the messages as text, to see them
```

What is played is the work's `render midi`, or, if it has none, the layers
of its first `render sound` or video `sound:`. With `--sync`, the music's
beats follow the DAW's tempo, a stop silences it and a start begins again;
with `--loop`, each new edition starts on the next bar. Ctrl-C silences
every note. Live MIDI needs alea built with `cargo build --release
--features midi` (on Linux, after `sudo apt install libasound2-dev`).

In the playground, a `render midi` shows its notes as a piano roll, plays a
rough preview in the page, plays to a MIDI device with Web MIDI (Chrome,
Edge), and saves the `.mid` file.

---

## 26. Harmony and rhythm: a generative track

> Tour: `26-harmony.alea`

Music lives in bars, so the first thing is a **meter**:

```alea
let m = meter(100, bars: 8)          // 100 beats per minute, 8 bars of 4 beats
render sound(music, duration: m.length)
render midi(music, duration: m.length, tempo: m.tempo)
```

`m.length` is the duration in seconds, `m.bar` and `m.beat` the length of
one bar and one beat. Keys can now be written as **note names** everywhere:
`key: "D3"`, `"F#2"`, `"Bb4"` (A4 is 440 Hz).

**Rhythms** are patterns, so they go wherever patterns go (`notes`, `arp`,
pictures, MIDI). Each spans the work, bar after bar (`bars:` a number or a
meter), and `y:` sets the height of its events (for drums: low is a kick,
then snare, hats, cymbals):

- **`euclid(hits, steps)`**: hits spread as evenly as possible over the
  steps of a bar. A small rule behind many of the world's rhythms:
  `euclid(3, 8)` is the tresillo, `euclid(5, 8)` the cinquillo, `euclid(7, 16)`
  a samba line. `rotate:` shifts it.
- **`steps("X...x...X...x..o")`**: a step sequencer. `X` an accent, `x` a
  hit, `o` a soft hit, `.` a rest; `|` starts the next bar, and the bars
  written repeat.
- **`steps(16, density: field)`**: a random hit on each of 16 steps, more
  likely where the field is high (read left to right along the middle of the
  canvas as time runs): a visual decides the groove.

On `notes`, **`swing: 0.6`** delays every second sixteenth (0.5 is straight,
0.66 a triplet feel) and **`humanize: 0.3`** loosens moments and velocities.
Both need the layer's `tempo`.

**Harmony.** `chords` makes a progression that spans the work:

```alea
let prog = chords("i iv V7 i", key: "A3", scale: "minor", bars: m)
let prog = chords("Am F C G7", bars: m)                       // chord names work too
let prog = chords("functional", key: "D3", scale: "dorian", bars: m, tension: 0.5)
```

- **Roman numerals** count on the scale: upper case for major, lower case
  for minor, `°` diminished, `+` augmented, `7`, `maj7`, `9`, `6`, `sus2`,
  `sus4`, and `b` or `#` in front for borrowed chords (`bVII`). `"i:2 iv V"`
  gives the first chord twice the time.
- **`"functional"`** draws the progression, one chord per bar, the way
  tonal music moves: home (tonic: I, vi, iii), away (subdominant: IV, ii),
  tension (dominant: V, vii°), and home again, with a cadence at the end of
  each phrase of four bars and V–I at the end. `tension` (0–1) asks for more
  dominants and sevenths; in a minor mode the dominant is often made major.
  Each edition gets a new progression that still sounds right.
- `prog.names` lists its chords ("i", "IV", "V7"…), and `prog.index` is a
  field of time, 0 for the first chord, 1 for the second…: pictures can
  follow the harmony.

Layers follow it with **`harmony: prog`**: a `drone`'s voices move to the
nearest note of each new chord (voice leading, gliding), and `notes` play
the notes of the chord of the moment (the height of an event picks among
them, over three octaves). The layer's `key` keeps its register: `key:
prog.key / 4` is two octaves down, a bass.

**The arpeggiator** plays the chord one note at a time, on a rhythm:

```alea
arp(prog, euclid(7, 16, bars: m), mode: "updown", octaves: 2, gate: 0.6, instrument: "marimba")
```

`mode` is `"up"`, `"down"`, `"updown"`, `"random"` or `"converge"` (from the
outside in); `octaves` how far it climbs; `gate` how long each note is held
before the next (in MIDI). It's a layer like `notes`: heard in alea's sound,
written in `render midi`, played live with `--midi`, following a DAW with
`--sync`. Its tempo comes from the meter of the progression.

---

## How values become media

Every medium receives the same thing, a field, and translates it:

| Medium | The field's value becomes |
|---|---|
| image, video | a colour from the palette |
| sound | the loudness of each voice over time |
| sculpture | a height |

Values are stretched to the range they actually take in this render (a few
extreme points at either end are ignored, so one bright spot doesn't wash out
the rest). The absolute values don't matter much; their differences do.
`circle() * 10` looks like `circle()`; `circle() + stripes() * 0.1` shows the
stripes faintly.

**Palettes** run from the lowest value to the highest (chapter 20 shows
how to make your own). `ink` is printing:
paper for low values, ink for high ones, so shapes (which are 1) come out
dark. The others (`sea`, `ember`, `lichen`, `night`) run from dark to light,
so shapes come out bright. That's also why a photo looks like a negative in
`ink`: wrap it in `invert(…)`.

---

## Tips

- **Start from "alea decides" and take control step by step.** `circle()`
  first, then give it a position once you know where you want it.
- **Bind what you want to share.** A palette chosen once with
  `let colours = choose { … }` and given to every render keeps the image and
  the video in agreement.
- **Use a seed while you work**, so you're comparing like with like:
  `--seed 0x2a`. In the playground, *Live* mode does this for you.
- **Make several editions** to see the range of a work:
  `alea work.alea -n 12`.
- **Read the errors.** They point at the line and say what was expected:

  ```
  error: `circle` has no parameter `radius` — it has: x, y, size, softness
    --> work.alea:2:26
  ```
