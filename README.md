# cbzViewer

Online CBZ and PDF file viewer.

## For Developers

### Generate CREDITS and legal.html

Install [`gocredits`](https://github.com/Songmu/gocredits) and run:

```bash
gocredits -skip-missing -w
```

> [!NOTE]
> Do not remove `CREDITS.2`.
> This contains some library which does not contain license info.

And then copy that info to `legal.txt`.

```bash
cat CREDITS > templates/legal.txt
cat CREDITS.2 >> templates/legal.txt
```