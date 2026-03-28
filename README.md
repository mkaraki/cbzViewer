# cbzViewer

Online CBZ and PDF file viewer.

## For Developers

### Generate CREDITS and legal.html

Write all frontend project's dependency to `CREDITS.3` and Write all backend project's dependency to `CREDITS.1`

And then copy that info to `legal.txt`.

```bash
cat CREDITS.1 > frontend/public/assets/legal.txt
cat CREDITS.3 >> frontend/public/assets/legal.txt
```