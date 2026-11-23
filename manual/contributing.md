# Contribute to This Wiki

The wiki is built from the `manual/` Markdown files in the main repository. Each page has an edit link. Propose corrections through a pull request, especially when a tool name or workflow changes.

## Preview locally

```sh
python -m pip install -r requirements-docs.txt
python -m mkdocs serve
```

Open the local address printed by MkDocs. Before submitting, run:

```sh
python -m mkdocs build --strict
python scripts/docs/check_site.py
```

## Writing a tutorial

State the result, starting document, tool locations, numbered steps and a visible checkpoint. Explain how to recover from the likely mistake. Use screenshots from the actual app and describe what the reader should notice. Do not use mockups as evidence that a feature exists.

The owl lessons include native projects, actual headless editor captures, rendered poses and labeled diagrams. Their reproducible generation instructions are in `manual/assets/README.md` in the repository. Keep captures separate from explanatory diagrams and describe sampled animations as teaching previews, not performance measurements.

## Publish to GitHub Pages

A repository administrator must select **Settings → Pages → Build and deployment → Source: GitHub Actions**. After the documentation changes are merged, run **Publish LibreToon manual** in Actions. Its deployment job publishes the generated site to GitHub Pages.

Pull requests build the wiki for validation without deploying it. Publication is a separate manual action. The site address is [alvillafm.github.io/LibreToon](https://alvillafm.github.io/LibreToon/).
