#!/usr/bin/env python3
"""Gera src-tauri/data/cities.tsv.gz a partir do GeoNames (CC BY 4.0, https://www.geonames.org/).

Colunas: nome, estado/região, país (ISO), latitude, longitude, fuso IANA, população, nomes alternativos.
Uso: python3 scripts/build_cities.py [cities1000|cities5000|cities15000]
"""
import gzip, io, pathlib, sys, unicodedata, urllib.request, zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASE = "https://download.geonames.org/export/dump/"
dataset = sys.argv[1] if len(sys.argv) > 1 else "cities1000"


def fetch(name):
    with urllib.request.urlopen(BASE + name, timeout=300) as r:
        return r.read()


def latin(s):
    return all(unicodedata.name(c, "").startswith("LATIN") or not c.isalpha() for c in s)


admin1 = {}
for line in fetch("admin1CodesASCII.txt").decode().splitlines():
    code, name, *_ = line.split("\t")
    admin1[code] = name

raw = zipfile.ZipFile(io.BytesIO(fetch(dataset + ".zip"))).read(dataset + ".txt").decode()
rows = []
for line in raw.splitlines():
    c = line.split("\t")
    name, ascii_name, alts, lat, lon, country, adm1, pop, tz = c[1], c[2], c[3], c[4], c[5], c[8], c[10], int(c[14] or 0), c[17]
    if not tz:
        continue
    # Nomes alternativos só para cidades grandes: é onde "Lisboa"/"Lisbon" faz diferença.
    extra = []
    if pop >= 100_000:
        seen = {name.lower(), ascii_name.lower()}
        for a in alts.split(","):
            if a and latin(a) and a.lower() not in seen and len(a) > 2 and not a.isupper():
                seen.add(a.lower())
                extra.append(a)
    if ascii_name.lower() != name.lower():
        extra.insert(0, ascii_name)
    rows.append((pop, "\t".join([name, admin1.get(f"{country}.{adm1}", ""), country, f"{float(lat):.4f}", f"{float(lon):.4f}", tz, str(pop), "|".join(extra[:60])])))

rows.sort(key=lambda r: -r[0])
out = ROOT / "src-tauri/data/cities.tsv.gz"
with gzip.open(out, "wt", encoding="utf-8", compresslevel=9) as f:
    f.write("\n".join(r[1] for r in rows))
print(len(rows), "cidades,", out.stat().st_size // 1024, "KB")
