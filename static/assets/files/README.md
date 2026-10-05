# Files

Slides, a CV, posters, and other files you want to host yourself go here.
Everything in `static/` is copied to the site as-is, so

```text
static/assets/files/my-talk.pdf
```

is served at `/assets/files/my-talk.pdf`, and linked from `content/site.toml`
with that root-relative path:

```toml
links = [{ label = "slides", url = "/assets/files/my-talk.pdf" }]
```

Root-relative links keep working when the site lives under a subpath such as
`https://you.github.io/homepage/` — the generator adds the prefix.

README files in `static/` are notes for you and are not published.
