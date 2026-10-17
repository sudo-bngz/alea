# alea reference

The complete rules of the language: syntax, statements, values, every
primitive, function and render, the command line and the limits. To learn
the language, start with the [guide](guide.md).

Contents: [Files](#files) · [Lexical rules](#lexical-rules) ·
[Grammar](#grammar) · [Statements](#statements) · [Expressions](#expressions) ·
[Values](#values) · [Chance](#chance) · [Primitives](#primitives) ·
[Functions](#functions) · [Lists](#lists) · [Your own functions](#your-own-functions) ·
[Patterns](#patterns) · [Growing](#growing) · [Paintings](#paintings) · [Renders](#renders) ·
[Components](#components) · [Command line](#command-line) · [Limits](#limits) ·
[Reproducibility](#reproducibility)

---

## Files

A source file has the extension `.alea`, is UTF-8 text, and holds exactly one
work, optionally preceded by `use` lines:

```alea
use "other.alea"          // zero or more

work Name(param = default) {
  statement
  statement
}
```

Nothing may follow the closing brace. Paths in `use`, `picture` and `font`
are relative to the folder of the file that contains them.

---

## Lexical rules

**Comments** start with `//` or `#` and run to the end of the line.

**Whitespace and newlines** separate tokens and are otherwise ignored, with
one exception: in a call, the `(` must be on the same line as the name.

**Names** start with a letter (any alphabet) or `_`, and continue with
letters, digits or `_`: `petal`, `étoile`, `size_2`. Case matters; by
convention works are capitalized (`Moon`) and everything else is lowercase.

**Numbers** are decimal: `3`, `0.25`, `1_000` (underscores are ignored). A
decimal point needs a digit on both sides (`0.5`, not `.5`). There is no
exponent notation. Negative numbers are written with the minus operator: `-2`.

**Texts** are written between double quotes on one line: `"ember"`. They
have no escape sequences and can't contain a double quote. `{expr}` inside a
text puts a value there (a fixed number, or a text); `{expr:02}` pads a whole
number with zeros to that many digits: `"{h:02}:00"` with h = 8 is `08:00`.
Other numbers are written with up to six decimals.

**Keywords** (reserved, can't be used as names):

```
use  work  let  field  pattern  constraint  render  output
repeat  event  choose  and  or  grow  then
```

**Built-in names** (can't be given to a `let`, `field`, `pattern` or parameter):

| Name | Meaning |
|---|---|
| `x`, `y` | the position on the canvas, 0 to 1, from the top-left |
| `t` | time, 0 to 1 over a video or a sound; 0 (or `at`) in an image |
| `i`, `n` | inside a pattern's event: the event's index (0 to n − 1) and the number of events |
| `pi`, `tau` | π and 2π |

`output` is a keyword, but after an `output` statement it can also be read as
a value (`render image(output)`). `for` and `in` (in a list loop), and `over`
and `for` (after `grow`), are only special in those places.

**Symbols**: `{ } ( ) [ ] , . : = + - * / % ^ < > <= >= == != ~ ~~ |`

---

## Grammar

```ebnf
program     = { "use" STRING } "work" NAME [ params ] "{" { statement } "}" ;
params      = "(" [ param { "," param } [ "," ] ] ")" ;
param       = NAME [ "=" expr ] ;

statement   = "let" NAME "=" expr
            | "let" NAME params "=" expr                      (* a function *)
            | "field" NAME "=" expr
            | "pattern" NAME "=" "repeat" expr "event" "(" [ args ] ")"
            | "constraint" expr
            | "render" NAME "(" [ args ] ")"
            | "output" expr
            | "grow" [ "over" expr ] [ "for" expr ] "{" { NAME "=" expr "then" expr } "}" ;

args        = arg { "," arg } [ "," ] ;
arg         = [ NAME ":" ] expr ;

expr        = expr binop expr | unop expr | postfix ;
postfix     = primary { "[" expr "]" | "." NAME } ;
binop       = "or" | "and" | "<" | ">" | "<=" | ">=" | "==" | "!="
            | "+" | "-" | "*" | "/" | "%" | "^" ;
unop        = "-" | "~" | "~~" ;
primary     = NUMBER | STRING | NAME
            | NAME "(" [ args ] ")"
            | "(" expr ")"
            | "choose" "{" branch { "|" branch } "}"
            | "[" [ expr { "," expr } [ "," ] ] "]"
            | "[" expr ( "for" NAME "in" expr )+ "]" ;
branch      = [ expr ":" ] expr ;
```

In an argument list, unnamed arguments must come before named ones. In
`list[i]`, the `[` must be on the same line as what it indexes.

---

## Statements

Statements run from top to bottom. A name can only be used after the
statement that gives it; giving a name again replaces it from that point on.

| Statement | Effect |
|---|---|
| `let name = expr` | Names any value: number, field, distribution, text, pattern, list. Every `~` in `expr` is drawn now, once. |
| `let name(params) = expr` | Defines a function. See [Your own functions](#your-own-functions). |
| `field name = expr` | Like `let`, but the value must be a number, a field or a pattern (not a text or a distribution). |
| `pattern name = repeat N event(attrs)` | Makes a pattern of `N` events. See [Patterns](#patterns). |
| `constraint expr` | The edition is valid only if `expr` is true (not 0). If not, the whole edition is drawn again. `expr` must be a fixed value. |
| `render kind(args)` | Produces a file. `kind` is `image`, `sound`, `video` or `sculpture`. See [Renders](#renders). |
| `output expr` | What the work gives when another work calls it. At most one per work; it must be a field or a pattern. |
| `grow over N for S { name = start then next … }` | Fields that evolve generation by generation on a grid. See [Growing](#growing). |

A work must contain at least one `render`, unless it's only used as a
component (then it needs an `output`).

---

## Expressions

### Precedence

From the loosest to the tightest binding:

| Level | Operators | Associativity |
|---|---|---|
| 1 | `or` | left |
| 2 | `and` | left |
| 3 | `<` `>` `<=` `>=` `==` `!=` | left |
| 4 | `+` `-` | left |
| 5 | `*` `/` `%` | left |
| 6 | unary `-` | prefix |
| 7 | `^` | right |
| 8 | `~` `~~` | prefix |

So `-2 ^ 2` is −4, `2 ^ 3 ^ 2` is 2⁹, and `~normal(0, 1) * 2` draws first and
then doubles. Use parentheses when in doubt.

### Operators

| Operator | Meaning |
|---|---|
| `a + b`, `a - b`, `a * b`, `a / b` | arithmetic, point by point |
| `a % b` | remainder, always between 0 and b for a positive b |
| `a ^ b` | a to the power b |
| `<` `>` `<=` `>=` `==` `!=` | 1 where true, 0 where false |
| `a and b` | 1 where both are non-zero |
| `a or b` | 1 where either is non-zero |
| `-a` | negation |
| `~d` | a draw from distribution `d` (see [Chance](#chance)) |
| `~~d` | a grain: a draw from `d` at every point |

Operators work on numbers and fields alike: if either side varies over the
canvas, so does the result. A division by zero gives an infinite or undefined
value, which renders as 0.

### Calls

`name(args)` calls, in this order of lookup: a work brought in with `use`,
then a [primitive](#primitives), then a [distribution](#chance), a
[measure](#measures) or a [function](#functions). Primitives and works accept
named arguments; functions, distributions and measures take unnamed ones only.

### Choices

```alea
choose { 3: a | 2: b | 1: c }
choose { a | b | c }
```

Picks one branch at random, in proportion to its weight (1 when omitted),
and evaluates only that branch. Weights must be fixed, zero or positive
numbers, not all zero. A branch can give any kind of value.

---

## Values

| Kind | Examples | Where it can be used |
|---|---|---|
| **number** | `3`, `~between(0, 1)`, `pi` | anywhere a field is expected, and wherever a fixed value is required |
| **field** | `x`, `sin(x * 10)`, `circle()` | arithmetic, functions, primitives, renders |
| **pattern** | `repeat 50 event(…)`, `scatter()` | as a field (each event a soft spot), in `count`, as the second argument of `sound` / `video` |
| **distribution** | `normal(0, 1)` | only behind `~` or `~~`, or named with `let` |
| **text** | `"night"` | render options (`palette`, `format`…), `text`, `picture`, `font`, `channel`, `axis`, parameters, `choose` |
| **list** | `[1, 2, 3]`, `[0.3, 0.5]` (a point) | indexing, loops, arithmetic (element by element), list functions, `merge`, `path`… |
| **function** | `let f(a) = …` | calls only |
| **painting** | `paint(f, palette: "sea")` | `mosaic`, `place`, `overlay`, `render image`, `render video`, `output` (not arithmetic) |
| **sound** | `drone(f)`, `notes(p)`, `texture(f)` | `render sound`, a video's `sound:` |

A number is a field that is the same everywhere. A **fixed value** is a
number that doesn't depend on `x`, `y`, `t` or a grain. Fixed values are
required by: distribution parameters, `choose` weights, constraints, a
pattern's count and event attributes, render options, `clouds(detail:)` and
`scatter(count:, size:)`.

---

## Chance

### Distributions

| Distribution | Parameters | Gives |
|---|---|---|
| `uniform(a, b)` | any a, b | a value from a to b, all equally likely |
| `between(a, b)` | | same as `uniform` |
| `normal(m, d)` | d > 0 | values around m, with spread d (about 68 % within d) |
| `around(m, d)` | | same as `normal` |
| `beta(a, b)` | a > 0, b > 0 | a value from 0 to 1; average a / (a + b) |
| `poisson(λ)` | λ > 0 | a whole number, average λ |
| `exponential(λ)` | λ > 0 | a positive value, average 1 / λ |

A distribution is a value of its own: `let d = normal(0, 1)`, then `~d`.

### When draws happen

`~d` draws one number at the moment its expression is evaluated:

| Written in | Drawn |
|---|---|
| a `let`, `field`, `constraint`, `render` or `output` | once per edition |
| a pattern's event attributes | once per event |
| the count of `repeat` | once per edition |
| a work parameter's default | once per call of that work |
| a `choose` branch | only if that branch is chosen |
| a primitive's omitted parameter | once per call of that primitive |

A name bound with `let` keeps its value: `let r = ~d` used twice is the same
number twice; `~d` written twice is two numbers.

`~~d` gives a **grain**: a field whose value at each point `(x, y, t)` is a
separate draw from `d`. It changes from point to point and from frame to
frame, but is fixed by the edition's seed.

---

## Primitives

Shared conventions:

- Positions and sizes are fractions of the canvas: `x`, `y` from 0 to 1, from
  the top-left; `size: 0.5` is half the canvas width.
- Angles are in degrees; positive angles turn clockwise.
- Shapes are 1 inside and 0 outside. Textures range from 0 to 1.
- **An omitted parameter is decided by alea**, as shown in the tables: a
  range means a fresh random value for each call; a single value is a fixed
  default. Except for `clouds(detail:)` and `scatter`, every parameter can
  itself be a field, for example `size: 0.2 + 0.1 * t`.
- Arguments can be given in the order listed, or by name.
- `softness` on shapes is the width of the edge, in canvas units.

### Shapes

| Primitive (parameters in order) | Omitted |
|---|---|
| `circle(x, y, size, softness)` — `size` is the diameter | x, y 0.25–0.75 · size 0.15–0.45 · softness 0.003 |
| `ring(x, y, size, width, softness)` — `size` is the diameter, `width` the line | x, y 0.25–0.75 · size 0.2–0.6 · width 0.02 · softness 0.003 |
| `rect(x, y, width, height, angle, softness)` — centred on x, y | x, y 0.25–0.75 · width, height 0.15–0.45 · angle 0–90 · softness 0.003 |
| `square(x, y, size, angle, softness)` | x, y 0.25–0.75 · size 0.15–0.4 · angle 0–90 · softness 0.003 |
| `line(x1, y1, x2, y2, width, softness)` — a segment | ends 0.1–0.9 · width 0.01 · softness 0.002 |
| `blob(x, y, size, wobble, softness)` — an irregular round shape; `wobble` from 0 (a circle) up | x, y 0.25–0.75 · size 0.2–0.45 · wobble 0.35 · softness 0.003 |

### Textures

| Primitive | Omitted |
|---|---|
| `stripes(count, angle, softness)` — `count` stripes across; angle 0 gives vertical stripes, 90 horizontal; softness from 0 (sharp) to 1 (smooth) | count 4–16 (whole) · angle 0–180 · softness 0.3 |
| `waves(count, angle, height, wiggles, softness)` — wavy stripes; `height` of the waves, `wiggles` along each stripe | count 4–12 · angle 0–180 · height 0.01–0.04 · wiggles 1–5 · softness 0.3 |
| `ripples(x, y, count, softness)` — rings around x, y | x, y 0.25–0.75 · count 5–20 · softness 0.3 |
| `dots(count, size, softness)` — a grid of `count` × `count` dots; `size` relative to a cell | count 5–15 · size 0.5 · softness 0.03 |
| `checker(count)` — a `count` × `count` checkerboard | count 4–12 |
| `gradient(angle)` — 0 to 1 across the canvas; angle 0 goes left to right | angle 0–360 |
| `glow(x, y, size)` — a soft light, brightest at x, y | x, y 0.25–0.75 · size 0.2–0.6 |
| `clouds(scale, detail)` — smooth noise; `scale` features across, `detail` layers (whole, 1–8, fixed) | scale 2–6 · detail 4 |
| `grain(amount)` — random at every point, 0 to `amount` | amount 1 |
| `scatter(count, size)` — a pattern of random specks (fixed values); each speck's size varies from 0.5 to 1.5 times `size` | count 20–80 · size 0.012 |

### Transforms

The first argument is what is transformed: any shape, texture, picture,
pattern or field.

| Primitive | Omitted |
|---|---|
| `move(shape, x, y)` — shifts by x, y | x, y −0.25–0.25 |
| `rotate(shape, angle, x, y)` — turns around x, y | angle 0–360 · x, y 0.5 |
| `zoom(shape, by, x, y)` — scales around x, y; `by: 2` is twice as big | by 0.5–2 · x, y 0.5 |
| `tile(shape, count)` — the whole canvas repeated `count` × `count` times | count 2–6 |
| `mirror(shape, axis)` — adds the reflection across the vertical centre line (`"vertical"`), the horizontal one (`"horizontal"`), or both (`"both"`) | axis `"vertical"` |
| `kaleidoscope(shape, sides, x, y)` — repeats the wedge to the right of x, y all around it, mirrored, `sides` times | sides 3–12 · x, y 0.5 |
| `warp(shape, amount, scale)` — bends space; `amount` is how far points move, `scale` how many bends across | amount 0.02–0.08 · scale 2–5 |

### Combining

| Primitive | Gives |
|---|---|
| `merge(a, b, …)` | the highest value at each point: everything (two or more arguments) |
| `overlap(a, b, …)` | the lowest value at each point: only where all are (two or more) |
| `cut(shape, by)` | `shape` with `by` removed: shape × (1 − by) |
| `invert(shape)` | 1 − shape |
| `threshold(shape, level, softness)` — a crisp edge where `shape` crosses `level` | level 0.5 · softness 0.02 |

Also: `mix(a, b, amount)` in [Functions](#functions), and plain arithmetic.

### Your own content

These two default to readable: level text, a picture filling the width.

| Primitive | Omitted |
|---|---|
| `text(words, x, y, size, angle, font)` — one line of text centred on x, y; `size` is the line's height | x, y 0.25–0.75 · size 0.06–0.15 · angle 0 · font `"sans"` |
| `picture(file, x, y, size, angle, channel)` — an image centred on x, y; `size` is its width, the height follows its proportions | x, y 0.5 · size 1 · angle 0 · channel `"luminance"` |

- `words` and `file` are required texts. Text is a single line.
- `font`: `"sans"`, `"bold"`, `"serif"`, `"mono"` (built in), or the path to
  a `.ttf` / `.otf` file.
- `picture` reads PNG, JPEG and WebP; larger than 4096 pixels, it is
  scaled down. `channel`: `"luminance"` (brightness), `"red"`, `"green"`,
  `"blue"`, or `"alpha"` (transparency, to use a cut-out as a shape).
  Transparent areas count as 0.
- Outside the text's or picture's rectangle, the value is 0.

---

## Functions

All take unnamed arguments and work on numbers and fields.

| Function | Gives |
|---|---|
| `sin(a)` `cos(a)` `tan(a)` | trigonometry, `a` in radians (`tau` is a full turn) |
| `abs(a)` `sign(a)` | absolute value; −1, 0 or 1 |
| `sqrt(a)` `exp(a)` `ln(a)` | square root, eᵃ, natural logarithm |
| `floor(a)` `fract(a)` | rounded down; what is after the decimal point (`a − floor(a)`) |
| `min(a, b)` `max(a, b)` | the smaller, the larger |
| `atan2(y, x)` | the angle of the direction (x, y), in radians |
| `step(edge, a)` | 1 where a ≥ edge, else 0 |
| `clamp(a, lo, hi)` | a kept between lo and hi |
| `mix(a, b, amount)` | a + (b − a) × amount |
| `smoothstep(e0, e1, a)` | 0 below e0, 1 above e1, a smooth curve between |
| `distance(x1, y1, x2, y2)` | the distance between two points; `distance(x, y, 0.5, 0.5)` is the distance to the centre |
| `noise(x, y, z)` | smooth noise between −1 and 1, different for each edition; vary z (for example with `t`) to animate it |

### Measures

Fixed values computed from a whole field, for constraints (and anywhere a
number is needed):

| Measure | Gives |
|---|---|
| `mean(f)` | the average value |
| `stddev(f)` | how much the values vary |
| `minimum(f)`, `maximum(f)` | the lowest and highest values |
| `count(p)` | the number of events of pattern `p` |

`mean`, `stddev`, `minimum` and `maximum` sample the field on a 24 × 24 grid
at four moments (t = 0, ⅓, ⅔, 1).

---

## Lists

| Written | Gives |
|---|---|
| `[a, b, c]` | a list (any values, lists included; a trailing comma is allowed) |
| `[expr for name in list]` | a list with `expr` computed for each element, bound to `name` |
| `[expr for a in l1 for b in l2]` | loops inside loops (the last one changes fastest) |
| `list[i]` | element `i`: 0 is the first, −1 the last; `i` must be a fixed whole number inside the list |
| `point.x`, `point.y` | `point[0]`, `point[1]` |

A **point** is a list of two values, `[x, y]`. In a loop, each `~` is drawn
again for every element, in order. Lists hold at most 100 000 elements.

**Arithmetic** (`+ - * / % ^`, comparisons, `and`, `or`, unary `-`) and the
[functions](#functions) work element by element: two lists must have the same
length; a single value applies to every element. `mix(a, b, 0.5)` on two
points is the point halfway between them.

| Function | Gives |
|---|---|
| `range(n)`, `range(a, b)` | the whole numbers 0 … n − 1, or a … b − 1 |
| `count(list)` | the number of elements |
| `sum(list)`, `mean(list)`, `minimum(list)`, `maximum(list)` | over the elements (numbers or fields); `stddev(list)` too |
| `merge(list)`, `overlap(list)` | of all the shapes in the list |
| `join(a, b, …)` | the lists end to end (a value that is not a list joins as one element) |
| `slice(list, from, to)` | elements `from` to `to` − 1; negative positions count from the end |
| `reverse(list)` | the same elements, last first |
| `distance(p, q)` | the distance between two points |
| `path(points, width, softness)` | the line through the points (at least 2), with round joints; width 0.01 · softness 0.002 |
| `length(points)` | the length of that line |
| `along(points, d)` | the point at distance `d` along it, from the start (clamped to the ends) |

---

## Your own functions

```alea
let snap(v, step = 24) = floor(v * step + 0.5) / step
```

- The body is one expression, computed at each call. Each `~` in it is drawn
  at each call.
- A function sees the names given **before** its definition, and its own
  parameters; not the names given after it, and not itself (so it can't call
  itself).
- Arguments are given in order or by name, like a primitive's. A parameter
  with a default can be left out; the default is computed at each call and
  can use the parameters before it.
- A function is called before anything else of the same name (a work brought
  in with `use`, a primitive, a function of the language).

---

## Patterns

```alea
pattern name = repeat count event(x: …, y: …, t: …, intensity: …, size: …)
```

- `count` is a fixed whole number from 0 to 100 000 (it can be drawn:
  `repeat ~poisson(60)`).
- The attributes are evaluated once per event, so each `~` in them is drawn
  again for every event. They must be fixed values (not depend on `x`, `y`,
  `t` or a grain).
- Inside the attributes, `i` is the event's index (0 to n − 1) and `n` the
  number of events.
- All attributes are optional:

| Attribute | Meaning | Omitted |
|---|---|---|
| `x`, `y` | position | uniform from 0 to 1 |
| `t` | moment, from 0 to 1 | uniform from 0 to 1 |
| `intensity` | strength | 1 |
| `size` | spread of the event's spot (and length of its note) | 0.02 |

**As a field**, a pattern is the sum of one soft round spot per event:
`intensity` at its centre, fading over a radius of about 2 × `size`.

**In a sound** (second argument of `sound` or `video`), each event is a
bell-like note: see [sound](#sound).

---

## Growing

```alea
grow over 160 for 360 {
  cells = ~~uniform(0, 1) < 0.3
    then neighbours(cells) == 3 or (cells and neighbours(cells) == 2)
  trace = 0
    then max(trace * 0.95, cells)
}
```

| Part | Meaning | Omitted |
|---|---|---|
| `over N` | the grid has N × N cells over the unit square; whole, 4–1024, fixed | 128 |
| `for S` | the number of generations after the first; whole, 1–100 000, fixed | 200 |
| `name = start then next` | one grown field (at least one line; names distinct) | |

**How it runs.** When the edition is drawn, every `start` is computed at the
centre of every cell (`t` = 0): that is generation 0. Then, S times, every
`next` is computed at every cell from the previous generation: inside `next`,
each grown name (its own and the other lines') means the previous generation.
`t` inside `next` is the moment of the generation being left, g / S. A `~` in
a rule is drawn once; a `~~` gives each cell, at each generation, its own
draw. Values that are not finite become 0. In `start`, the block's own
names still mean what they meant before the block, if anything.

**Afterwards**, each name is a field of x, y and t: `t` = 0 is the first
generation and `t` = 1 the last. Between cells the values are blended
(bilinearly), between generations too (linearly), and the grid wraps around
on all sides (a torus): the canvas outside the unit square repeats it.

**Neighbourhood functions**, only inside `next`, on any field (usually a
grown name), measured in cells of this grid:

| Function | Gives |
|---|---|
| `neighbours(f)` (or `neighbors`) | the sum of `f` over the 8 surrounding cells |
| `spread(f)` | the discrete Laplacian: 0.2 × each side cell + 0.05 × each corner − the cell itself; adding it diffuses `f` |
| `nearby(f, r)` | the average of `f` over the cells within `r` cells (a disc, the cell included); `r` whole, 1–12, fixed |

**Kept generations.** At most 600 generations are kept, evenly spaced, and
no more than about 64 million values in all (cells × fields × generations
kept); the others are computed but not kept, so a video moves through the
kept ones. Rules that don't change the work (a chemistry map, a mask) are
computed once, not at every generation.

---

## Paintings

A painting is a field in colour, or paintings laid out. Images and videos
show it in its own colours; it is computed as pixels, each part at the size
it occupies on the canvas.

| Function | Parameters (in order) | Gives |
|---|---|---|
| `paint` | `field`, `palette` ("ink", or a list of colours), `range` (measured), `alpha` (1), `bands` (0) | the field through the palette. `range: [low, high]` maps those values to the ends of the palette; without it, the range is measured once over the whole animation (ignoring the extreme 0.5 % at each end). `alpha` is a field, 0 (transparent) to 1 |
| `mosaic` | `paintings` (a list), `columns` (√n, rounded up), `gap` (0.01), `background` (0) | a grid filling the canvas: cells of equal size, row after row, `gap` apart (a fraction of the canvas's shorter side). Each cell shows a whole painting as a canvas of its own. `background` is a grey level (0–1) or a painting |
| `place` | `painting`, `x` (0.5), `y` (0.5), `width` (0.5), `height` (= width) | the painting in that rectangle (centred on x, y, in canvas units), transparent elsewhere |
| `overlay` | paintings (any number, or a list), then `blend` ("normal"), `opacity` (1), `mask` (1) | the first painting, then each next one over it. `blend`: normal, add, screen, multiply, lighten, darken, difference, overlay. `opacity` (0–1) and the `mask` field (0–1) say how much of each upper layer shows |

A field given where a painting is expected is painted in "ink". Where a
painting is transparent all the way down, the image is black. A work's
`output` can be a painting. A video of a painting has no sound unless it is
given `sound:` layers.

---

## Time and filters

Functions that take `time` use `t` when it is not given.

| Function | Parameters (in order) | Gives |
|---|---|---|
| `progress` | `start`, `end`, `time` (t) | clamp((time − start) / (end − start), 0, 1) |
| `ease` | `value`, `kind` ("in-out") | the value clamped to 0–1, through a curve: linear, in (cubic), out (cubic), in-out (smoothstep), sine, back (overshoots by about 10 %), elastic, bounce. On a list, each element |
| `keys` | `frames` (a list of `[time, value]`, times fixed and in order), `ease` ("linear"), `time` (t) | the first value before the first time, the last after the last, eased between each pair. Values can be numbers, fields or lists of the same shape (points) |
| `sequence` | `items` (fields or paintings), `fade` (0.1), `time` (t) | item i shows from i/n to (i+1)/n, crossfading over `fade` around each change (at most 1/n). If any item is a painting, a painting |
| `at` | `of` (a field or a painting), `time` | the field or painting evaluated with `time` in place of t, everywhere inside it |
| `lfo` | `times` (1), `shape` ("sine"), `phase` (0, in turns), `from` (0), `to` (1), `time` (t) | with u = fract(time × times + phase): sine 0.5 − 0.5 cos(2π(time × times + phase)); triangle 1 − abs(2u − 1); saw u; square step(0.5, u); pulse (1 − u)⁴; then scaled from `from` to `to`. A whole number of `times` loops over the work |
| `orbit` | `radius` (0.25, or [rx, ry]), `times` (1, or [tx, ty]), `phase` (0, in turns), `x` (0.5), `y` (0.5), `time` (t) | the point [x + rx cos(2π(time × tx + phase)), y + ry sin(2π(time × ty + phase))]: clockwise on the canvas (y grows downwards) |

**Field filters** give a field:

| Function | Parameters | Gives |
|---|---|---|
| `blur` | `of`, `radius` (0.01) | a Gaussian average over 40 points on a spiral, out to twice the radius |
| `pixelate` | `of`, `size` (0.02) | the value at the centre of each square cell |
| `edges` | `of`, `step` (0.002) | the length of the difference across ±step, in x and y |
| `relief` | `of`, `angle` (135), `depth` (10), `step` (0.003) | 0.5 + depth × (f towards the light − f away from it) |
| `posterize` | `of`, `levels` (6) | floor(clamp(f, 0, 1) × levels) / (levels − 1) |
| `film` | `of`, `amount` (0.08) | f + amount × a uniform grain in [−1, 1], different at every point and frame |
| `vignette` | `of`, `amount` (0.5) | f × (1 − amount × smoothstep(0.25, 0.75, distance to the centre)) |

**Painting filters** give a painting; they work on the pixels, at the size
the painting is drawn:

| Function | Parameters | Gives |
|---|---|---|
| `blur` | `of`, `radius` (0.01, up to 0.3) | a separable Gaussian of that spread, as a fraction of the shorter side |
| `pixelate` | `of`, `size` (0.02) | the average of each square block |
| `film` | `of`, `amount` (0.08) | grain on each channel, new at every frame |
| `vignette` | `of`, `amount` (0.5) | darker towards the corners |
| `bloom` | `of`, `amount` (0.8, up to 4) | what is brighter than about half, blurred near and far, screened back on |
| `chroma` | `of`, `amount` (0.5, up to 4) | red pushed outwards and blue inwards from the centre, by 2 % × amount of the distance |

`bloom` and `chroma` given a field paint it in "ink" first; `edges`,
`relief` and `posterize` need a field.

---

## Inputs

`input(name, or)` gives what the work was given: the default input (`name`
omitted) comes from the pipe or `--in FILE`; a named one from
`--in NAME=FILE`. Without one, `or` is used: a text is a file to read
(relative to the work), anything else is used as it is. Without either, an
error. `or` is computed in every case, so the work makes the same draws with
or without an input.

An input is recognised by its first bytes: PNG, JPEG or WebP (an image), WAV
(a sound: PCM 8, 16, 24 or 32 bits, or float, at most two channels and ten
minutes), otherwise UTF-8 text. Inputs make no draw.

| Image | Gives |
|---|---|
| `.luma` (`.luminance`), `.red`, `.green`, `.blue`, `.alpha`, `.hue`, `.saturation`, `.lightness` | a field, 0 to 1 (transparent pixels give 0; hue goes round from red at 0). The image is centred and covers the canvas: its shorter side spans 0 to 1 |
| `.painting` | the image in its own colours, placed the same way |
| `.width`, `.height`, `.aspect` | its size in pixels, and height ÷ width |
| `colours(image, count)` (5) | a list of `count` colours (1–32), `"#rrggbb"`, darkest first: k-means on up to 16 000 of its pixels, always the same for the same image |

An image input sets the default size of images (its own size) and the
proportions of videos (`width` 720 and the image's proportions).

| Sound | Gives |
|---|---|
| `.level` | the loudness along the sound (t = 0 at its start, 1 at its end), 200 values per second, scaled so the loudest 1 % reach 1; quick to rise, slower to fall |
| `.bass`, `.mid`, `.treble` | the same through a band: 20–200, 200–2000, 2000–16 000 Hz (fourth-order filters) |
| `band(sound, low, high)` | the same for any band, in Hz |
| `.pulse` | each beat: its strength, then an exponential fade (0.18 s) |
| `.beats` | a pattern: one event per onset (a sudden rise in three bands, at least 0.1 s apart), at the time of the onset; intensity its strength (0.2–1); position drawn from the edition's seed (no draw is made) |
| `.duration` | seconds |
| `sound: [song]`, `render sound(song)` | the sound itself, as a layer: not through the room or the echo |

A sound input sets the default `duration` of videos and sounds (its own,
at most 600 s).

| Text | Gives |
|---|---|
| `.text` | all of it |
| `.words`, `.lines` | lists of texts (blank lines skipped) |
| `.rows` | a list per line: cells split on `,` `;` or tabs (otherwise spaces); a cell that reads as a number is a number |

A default given as a value stands in for the input: its channels and curves
(`.luma`, `.level`…) are itself, `.alpha` is 1, `.painting` paints it in
"ink", and `colours` gives "ink" colours.

---

## Music: meters, rhythms, harmony

| Function | Parameters (in order) | Gives |
|---|---|---|
| `meter` | `tempo` (120; 20–400), `bars` (8), `beats` (4) | a meter: `.length` = bars × beats × 60 ÷ tempo seconds (at most 600), `.tempo`, `.bars`, `.beats`, `.bar`, `.beat` (seconds) |
| `euclid` | `hits`, `steps` (1–64), `rotate` (0), `bars` (1, or a meter), `y` (0.5) | a pattern: in each bar, a hit on step k when ((k − rotate) × hits) mod steps < hits; t = x = the step's moment over the work; intensity 1 on the first hit of the cycle, 0.8 otherwise |
| `steps` | `pattern` (a text, or a number of steps 1–64), `bars` (1, or a meter), `y` (0.5), `density` (0.5) | a written pattern: `X` 1, `x` 0.65, `o` 0.35, `.` `-` `_` rest, spaces ignored, `\|` the next bar (the bars written repeat over `bars`). A number: a hit on each step with the probability of the field at (t, 0.5, t), clamped to 0–1 (intensity 0.5 + 0.5 × it); drawn from the edition's seed without a draw |
| `chords` | `progression` (a text), `key` ("A3", 220 Hz), `scale` ("major"; any scale of semitones), `bars` (4, or a meter: its tempo goes with the progression), `tension` (0.4), `repeat` (1) | a progression over the work. Written: chords separated by spaces, `,` or `\|`, each with an optional `:weight`, repeated `repeat` times, sharing the time by weight. `"functional"`: `bars` chords, see below |
| `arp` | `harmony`, `rhythm` (a pattern), `mode` ("up"), `octaves` (2; 1–5), `gate` (0.8), `instrument` ("pluck"), `key` (the progression's), `gain` (1), `tempo` (the progression's meter), `swing` (0.5), `humanize` (0) | a `notes` layer: the r-th event in time order plays step r of the mode over the chord's tones, ascending over `octaves` octaves from the key's register; held `gate` × the time to the next event (in MIDI) |

**Roman numerals** are counted on the scale (a scale of other than seven
notes uses the major or minor scale it is closest to): I–VII, upper case
major (0 4 7), lower case minor (0 3 7), then `°` (0 3 6; `°7` adds 9), `ø`
(0 3 6 10), `+` (0 4 8), `7` (adds 10), `maj7` (11), `9` (10 and 14), `6`
(9), `sus2`/`sus4` (the third becomes 2 or 5); `b`/`#` before the numeral
lower or raise the root. **Chord names**: a letter A–G, `#` or `b`, then
nothing (major), `m`, `dim`, `aug`, `7`, `maj7`, `m7`, `m7b5`, `dim7`,
`sus2`, `sus4`, `6`, `m6`, `9`, `m9` or `add9`; their roots are counted from
the key's note.

**`"functional"`**: chord 1 is I and the last is I, with V before it; the
others walk between functions, tonic (I 0.5, vi 0.35, iii 0.15),
subdominant (IV 0.55, ii 0.45) and dominant (V 0.8, vii 0.2), from tonic to
tonic 0.15, subdominant 0.5, dominant 0.35 + 0.2 × tension; from subdominant
to itself 0.15, dominant 0.6 + 0.2 × tension, tonic 0.25; from dominant to
tonic 0.8, itself 0.1 + 0.1 × tension, subdominant 0.1. The fourth chord of
each phrase is V with probability 0.6 (a half cadence). Chords are the
scale's own triads, with its seventh with probability tension × 0.8 for V,
tension × 0.35 otherwise; in a minor scale V is made major with probability
0.5 + 0.5 × tension. The draws use the edition's randomness.

**A progression's members**: `.names` (a list of texts), `.count`, `.key`
(Hz), `.index` (a field of t: 0, 1, 2… which chord is playing).

**`harmony:`** on `drone`: each voice starts on a tone of the first chord
(stacked upwards from the layer's key) and, at each change, moves to the
nearest pitch of the new chord that no other voice holds, gliding over 60 ms;
the default key is the progression's, an octave down. On `notes`: the
height of an event picks among the chord's tones over three octaves from the
layer's key (default: the progression's), the bottom of the canvas the
lowest. The progression gives the notes, the layer's key the register (the
key's pitch class is replaced by the progression's).

**`swing`** (0.5–0.75) and **`humanize`** (0–1) on `notes` and `arp`: on the
tempo's grid, every odd sixteenth moves by (swing − 0.5) × 2 sixteenths;
humanize moves each note by up to ± 30 ms × humanize and scales its
velocity by up to ± 40 % × humanize, from the edition's seed.

---

## Renders

`render kind(args)` writes one file. A work can have any number of renders;
all belong to the same edition. Files are named
`<work>-<seed>.<ext>`, with `-1`, `-2`… added when a work has several renders
of the same kind.

**How values become media.** Before being turned into colour, loudness or
height, values are stretched to the range they take in that render, ignoring
the most extreme 0.5 % at each end (for colour, `range: [low, high]` sets
the range instead). A field that is the same everywhere is
shown as it is (0 blank, 1 full).

**Canvas shape.** When width and height differ, the 0–1 square stays fully
visible and centred; the longer side shows more around it.

**Options** must be fixed values (texts for `palette`, `format`, `audio`).

### image

`render image(field, …)` → PNG

| Option | Default | Range |
|---|---|---|
| `width` | 1024 | 16–8192 pixels |
| `height` | = width | 16–8192 pixels |
| `at` | 0 | the moment `t` to show |
| `palette` | `"ink"` | a name or a list of colours: see [Palettes](#palettes) |
| `bands`, `range` | | see [Palettes](#palettes) |

### sound

`render sound(layer, layer, …)` → WAV, stereo, 44.1 kHz

The layers are sounds made by `drone`, `notes` and `texture` (or lists of
them); they are mixed, sent through an echo and a room, and mastered.

| Option | Default | Range |
|---|---|---|
| `duration` | 20 | 0.5–600 seconds |
| `room` | 0.25 | 0–1: the size of the space around the sound (0: none; 0.5: a hall, about a second of tail; 0.9: a cathedral) |
| `echo` | 0 | 0–1: a ping-pong echo, three eighths of a beat (of the notes' tempo, or 0.375 s) |

`key` can always be a note name ("A3", "C#4", "Bb2"; A4 = 440 Hz) as well as a frequency. With `harmony:`, see *Music* above.

**`drone(field, voices, instrument, scale, key, gain, harmony)`**: voices that hold
notes of a scale. Voice k (from 0) listens to the field along the line
y = (k + 0.5) / voices, at x = 0.3 and at x = 0.7, as `t` goes from 0 to 1:
it is loud where the field is high there, sounds towards the side where it is
higher, and its upper partials open as it gets louder. Voice k plays degree k
of the scale (every other degree for scales of six notes or more, so the
voices make chords); the first voice, at the top of the canvas, is the lowest.

| Parameter | Default | Values |
|---|---|---|
| `voices` | 6 | 1–24 |
| `instrument` | `"sine"` | `"sine"`, `"organ"`, `"reed"`, `"strings"` (three players, vibrato), `"choir"` (a vowel that opens with the field), `"glass"`, `"wind"` (breath, tuned) |
| `scale` | `"pentatonic"` | see below |
| `key` | 110 | 20–2000 Hz: the first note |
| `gain` | 1 | 0–10 |

**`notes(pattern, instrument, scale, key, gain, tempo, harmony, swing, humanize)`**: each event plays
one note. It starts at `t` × duration (on the nearest sixteenth note if
`tempo` is given, in beats per minute); its `y` picks the note, over three
octaves of the scale from the bottom of the canvas to the top; its `x`
places it between left and right; it rings for about 0.08 + 20 × `size`
seconds (at most 3, longer for bells and glass, shorter for wood and ticks);
`intensity` sets its loudness.

| Parameter | Default | Values |
|---|---|---|
| `instrument` | `"bell"` | `"bell"`, `"pluck"` (a string), `"marimba"`, `"kalimba"`, `"glass"`, `"wood"`, `"drum"` (two octaves down), `"tick"` |
| `scale` | `"pentatonic"` | see below |
| `key` | 220 | 20–4000 Hz |
| `gain` | 1 | 0–10 |
| `tempo` | 0 (free) | 0–400 beats per minute |

**`texture(field, kind, gain)`**: an ambience. Its loudness follows the
average of the field over the canvas as `t` runs (stretched to the range it
takes).

| Parameter | Default | Values |
|---|---|---|
| `kind` | `"wind"` | `"wind"` (brighter when louder), `"waves"` (slow swells, out of step between the ears), `"rain"` (drops, more of them), `"crackle"` (sparse pops) |
| `gain` | 1 | 0–10 |

**Scales**: `"pentatonic"`, `"minor-pentatonic"`, `"major"`, `"minor"`,
`"dorian"`, `"phrygian"`, `"lydian"`, `"mixolydian"`, `"harmonic-minor"`,
`"whole-tone"`, `"blues"`, `"hirajoshi"`, `"chromatic"` (equal temperament),
and `"harmonics"` (the first eight partials of the key, then the same an
octave up).

Each layer is brought to a comparable level before its `gain` applies; the
mix is normalized, gently limited, and faded in and out over half a second.

**The original form**, `render sound(field, pattern, fundamental:, voices:)`,
still works and sounds as before (mono): a sine drone on a five-note scale
(1, 9/8, 5/4, 3/2, 5/3 of `fundamental`, 110 Hz by default) listening at
x = 0.5, and bell notes whose pitch rises from 2 to 16 times the fundamental
with `y`.

### video

`render video(field, pattern?, …)` → MP4 (H.264, with AAC sound) or GIF.
Needs [ffmpeg](https://ffmpeg.org) on the command line.

| Option | Default | Range |
|---|---|---|
| `duration` | 10 | 0.5–600 seconds |
| `fps` | 30 | 1–120 frames per second |
| `width` | 720 | 16–3840 (rounded down to even) |
| `height` | = width | 16–3840 (rounded down to even) |
| `palette` | `"ink"` | |
| `format` | `"mp4"` | `"mp4"` or `"gif"` |
| `audio` | `"on"` for mp4, `"off"` for gif | `"on"` or `"off"` (a gif can't have sound) |
| `sound` | the original drone and bells | a sound or a list of sounds, as in `sound` |
| `room`, `echo` | as in `sound` | |
| `fundamental`, `voices` | as in the original `sound` | when there is no `sound:` |

`t` goes from 0 at the first frame towards 1 (frame k of N is at t = k / N),
so a work built on `sin(t * tau)` loops seamlessly. Colours use one scale for
the whole video, so they don't flicker. The soundtrack is the `sound:` layers if given,
otherwise the original drone and bells of the field and pattern.

### sculpture

`render sculpture(field, …)` → STL (binary), a closed solid, in millimetres

| Option | Default | Range |
|---|---|---|
| `resolution` | 160 | 8–1024 points per side |
| `side` | 100 | width and depth, in mm |
| `relief` | 0.25 | height of the tallest point, as a fraction of `side` |
| `base` | 4 | thickness of the base, in mm (at least 0.5) |
| `at` | 0 | the moment `t` to sculpt |

### Palettes

`palette` is a name or a list of colours.

| Palette | Colours |
|---|---|
| `ink` | paper → grey → ink (high values are dark) |
| `sea` | deep blue → blue → pale turquoise → foam white |
| `ember` | black → dark red → orange → pale yellow |
| `lichen` | dark moss → green → pale green → cream |
| `night` | black → violet → pink → white |
| `["#fff", "#f5d000", "#d40920"]` | your own: a gradient through these colours, evenly spread from the lowest value to the highest |
| `[[0, "#fff"], [0.6, "#fff"], [0.6, "#d40920"], [1, "#111"]]` | colours placed at values from 0 to 1, in order; two colours at the same value make a sharp edge |

Colours are written `"#rrggbb"` or `"#rgb"`. With these options (on `image`,
`video` and `paint`):

| Option | Default | Effect |
|---|---|---|
| `bands` | 0 (smooth) | 2–256: the values are cut into that many flat levels, from the first colour to the last. With as many bands as colours, each colour gets one band |
| `range` | measured | `[low, high]`: those values are the two ends of the palette, instead of the range the field takes (so `range: [0, 1]` maps a field's values as they are) |

---|---|
| `ink` | paper → grey → ink (high values are dark) |
| `sea` | deep blue → blue → pale turquoise → foam white |
| `ember` | black → dark red → orange → pale yellow |
| `lichen` | dark moss → green → pale green → cream |
| `night` | black → violet → pink → white |

### midi

`render midi(layers…, duration, tempo)` → a Standard MIDI File (format 1, 480 ticks per quarter note)

| Option | Default | Range |
|---|---|---|
| `duration` | 20 (or a sound input's) | 0.5–600 s |
| `tempo` | the first `notes` layer's `tempo`, else 120 | 1–400 BPM (the file's grid; the notes keep their moments in seconds) |

A first track holds the title, the tempo and 4/4; then one track per layer,
on its own channel (1, 2, 3…, skipping 10).

| Layer | Becomes |
|---|---|
| `drone` | a General MIDI program (sine → 89, organ → 19, reed → 68, strings → 48, choir → 52, glass → 92, wind → 91); each voice a note (the degree it holds), on when its loudness passes 0.06 (velocity 30 + 97 × √loudness), off below 0.02; controller 11 (expression) follows the loudest voice, 25 times a second |
| `notes` | a program (bell → 14, pluck → 24, marimba → 12, kalimba → 108, glass → 9, wood → 115); a note per event at its moment (on the tempo's grid if it has one), velocity 127 × intensity, held 90 % of its ring. `drum` and `tick` go to channel 10, the degree picking from the kit (36 kick, 41, 38 snare, 39, 45, 42 closed hat, 46 open hat, 48, 51 ride, 49 crash) or from ticks (37, 42, 75, 76, 77, 44) |
| `cc(field, number, channel)` | controller `number` (74; 0–127) on `channel` (1; 1–16): the field's mean over the canvas, clamped to 0–1, as 0–127, 50 times a second when it changes. Silent in a sound |
| `texture`, a sound input | nothing (they have no notes) |

Notes are the nearest MIDI notes to alea's pitches. For a scale of semitones
whose key falls between two notes, a pitch bend on the channel (range ±2)
puts the whole layer in tune; the harmonic series is rounded.

### Live MIDI

`--midi` plays the edition's `render midi` (else its first `render sound` of
layers, else its first video `sound:`) in real time, then stops; `--loop`
goes on with new editions (S, S+1… with `--seed S`), each prepared while the
one before plays. `--sync` waits for a DAW's start and follows its clock (24
ticks per beat): the music's beats at the DAW's tempo, its stop silences the
notes, a new start begins the edition again, and with `--loop` each edition
starts on the next bar. Ctrl-C, or the end, sends a note-off for every note
still on and "all notes off" (controller 123) on every channel used.
Needs the feature `midi` (except `--midi-port print`).

---

## Components

A work becomes a component when it has an `output`; another work brings it in
with `use` and calls it by name.

```alea
work Flower(cx = ~between(0.2, 0.8), size = 0.2, petals) { … output … }
```

- **Parameters** are names, optionally with a default. A default is evaluated
  for each call that doesn't give the parameter, and can use the parameters
  before it. A parameter without a default must be given by every caller,
  and makes the work unable to run on its own.
- **A call** `Flower(…)` takes arguments like a primitive (in order, or by
  name) and returns the work's `output`. It can be used anywhere a field is.
- **Every call is a new draw.** Its seed is drawn from the caller's, so the
  whole composition replays from one seed. Each instance has its own grain
  and noise; a field passed in as an argument keeps the caller's.
- **The component's constraints are retried locally**: only the component is
  drawn again.
- **Its renders are ignored** when it is called.
- **Its names are its own**: a component sees only its parameters and its own
  statements, not the caller's names.

### use

```alea
use "lib/flower.alea"
```

- Paths are relative to the file containing the `use`.
- A file only sees the works it `use`s itself.
- Two works with the same name, or a work with the same name as the file's
  own, is an error. So is a circular `use` (a uses b, which uses a).

---

## Command line

```
alea <file.alea> [options]
cat input | alea <file.alea> [options] > output
```

| Option | Effect |
|---|---|
| `-s`, `--seed N` | draw edition N (decimal or `0x…` hexadecimal) instead of a random one |
| `-n`, `--editions N` | draw N editions; with `--seed S`, editions S, S+1, … |
| `-o`, `--out DIR` | write files into DIR (default: the current folder). `-o -`: the first render to the standard output |
| `--in FILE` | the default input, instead of the pipe; `--in -` waits for the pipe however long it takes (otherwise, a pipe silent for 3 seconds counts as no input) |
| `--in NAME=FILE` | a named input, for `input("NAME")` |
| `--seed-from-input` | the seed is the input's fingerprint: the same input, the same edition |
| `--midi` | play the music live on a new virtual MIDI port, "alea" |
| `--midi-port P` | play to the existing port whose name contains P; `print` writes the messages as text |
| `--midi-ports` | list the MIDI ports |
| `--loop` | with `--midi`: a new edition after each one, until Ctrl-C |
| `--sync`, `--sync-port P` | with `--midi`: follow a DAW's MIDI clock, from a new port "alea clock" or from P |
| `--check` | check the work and draw it once (seed 0 unless `-s`), without writing files |
| `--web FILE` | write a web page that draws a new edition on every visit |
| `-h`, `--help` | show this list |

Each edition prints its seed, the fingerprint of each input it read, and the
files it wrote. The pipe is read only when the work asks for its input. When
the standard output is not a terminal and no `-o` is given, the first render
goes to it and the messages to the error output, so works chain:
`cat a.png | alea blur.alea | alea frame.alea > b.png`. Errors are printed with
the file, line and column, and a mark under the problem.

### In the browser

- `web/dist/playground.html` is the playground: an editor, the examples and
  the tour, and every render previewed in the page. It is one file and works
  offline.
- `alea work.alea --web page.html` writes a page for one work. A visit draws a
  random edition; an address ending in `#0x…` opens that edition.

Both run the same engine as the command line, compiled to WebAssembly, and
compute the same images, sounds and sculptures. Videos are previewed in the
browser; the MP4 file comes from the command line.

---

## Limits

| Limit | Value |
|---|---|
| attempts to satisfy the constraints | 1000 per edition (per component) |
| works nested inside works | 16 levels |
| events in a pattern | 100 000 |
| image size | 16–8192 pixels per side |
| video size | 16–3840 pixels per side |
| sound and video duration | 0.5–600 seconds |
| sculpture resolution | 8–1024 |
| picture size | scaled down beyond 4096 pixels |

---

## Reproducibility

An edition is fully determined by its work's files, its seed and its inputs. The same
seed gives byte-identical PNG, WAV and STL files on every operating system and
in the browser. MP4 and GIF files are encoded by ffmpeg, whose output can
vary between ffmpeg versions; their frames don't.

alea uses its own maths library and its own random samplers for this reason:
system maths can differ from one platform to another in the last bit, and a
draw must not.
