# alea documentation

- **[Guide](guide.md)**: learn alea in twenty-six chapters, each paired with a
  file of the tour in [`examples/tour/`](../examples/tour/).
  1. A work, and its editions · 2. Shapes, and what alea decides ·
  3. Chance · 4. Choices · 5. Combining · 6. Transforms · 7. Textures ·
  8. Words and pictures · 9. Fields · 10. Patterns · 11. Constraints ·
  12. Time · 13. Sound: layers · 14. Sculpture · 15. Works made of works ·
  16. Everything together · 17. Growing · 18. Lists and functions · 19. Paintings · 20. Colours of your own · 21. Animation · 22. Filters · 23. Input · 24. Listening · 25. MIDI · 26. Harmony and rhythm; then how values become media, and tips.
- **[Reference](reference.md)**: files, lexical rules, grammar, statements,
  expressions and precedence, values, distributions and when draws happen,
  every primitive with its parameters and defaults, functions and measures,
  lists, your own functions, patterns, growing, paintings, time and filters, inputs, meters, rhythms and harmony, the five renders and their options, live MIDI, palettes, components and
  `use`, the command line, limits, reproducibility.

- **[How alea works](architecture.md)**: the engine's parts, how it stays fast
  without changing a pixel, building the browser engine, and what comes next.

The same two documents (guide and reference), illustrated with the tour's renders, are built into
one web page by `python3 web/handbook.py` (→ `web/dist/handbook.html`).
