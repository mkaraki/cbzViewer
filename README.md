# cbzViewer

Online CBZ and PDF file viewer.

## For Developers

### Generate CREDITS and legal.html

Install [`cargo-about`](https://github.com/EmbarkStudios/cargo-about) and run:

```bash
cargo about generate CREDITS.hbs -o CREDITS
```

> [!NOTE]
> Write all frontend project's dependency to `CREDITS.3`

And then copy that info to `legal.txt`.

```bash
cat CREDITS > frontend/public/legal.txt
cat CREDITS.3 >> frontend/public/legal.txt
```