# Images

Put your portrait here — for example `static/assets/img/portrait.jpg` — and
point `content/site.toml` at it:

```toml
[person]
photo_path = "/assets/img/portrait.jpg"
photo_alt = "Portrait of Your Name"
```

Then delete the placeholder `portrait.svg`.

Use a square crop with the face centred. A 600×600 to 1200×1200 JPEG or PNG is
plenty; the generator reads the real dimensions and writes them into the
`<img>`, so the space is reserved before the portrait loads. Other formats
(WebP, AVIF, SVG) work too, just without `width` and `height`.

To show no portrait at all, remove `photo_path`.

README files in `static/` are notes for you and are not published.
