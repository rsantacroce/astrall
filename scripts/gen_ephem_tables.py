#!/usr/bin/env python3
"""Gera as tabelas embutidas de Plutão e Quíron e os valores de referência dos testes.

Fonte: JPL Horizons (https://ssd.jpl.nasa.gov/horizons/), dados de domínio público.
Só precisa ser reexecutado para ampliar o intervalo de datas.

  python3 scripts/gen_ephem_tables.py tables      # crates/astro-core/data/*.bin
  python3 scripts/gen_ephem_tables.py reference   # crates/astro-core/tests/horizons_reference.json
"""
import json, struct, sys, urllib.error, urllib.parse, urllib.request, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
API = "https://ssd.jpl.nasa.gov/api/horizons.api"
JD0, JD1, STEP = 2378480.5, 2524624.5, 32  # 1799-12-22 .. 2200-01-09, passo em dias


def horizons(**params):
    q = {"OBJ_DATA": "NO", "MAKE_EPHEM": "YES", "CSV_FORMAT": "YES"}
    q.update(params)
    url = API + "?format=text&" + "&".join(f"{k}={urllib.parse.quote(repr(v) if isinstance(v, str) else str(v))}" for k, v in q.items())
    try:
        with urllib.request.urlopen(url, timeout=120) as r:
            text = r.read().decode()
    except urllib.error.HTTPError as e:
        raise SystemExit(f"Horizons HTTP {e.code}: {e.read().decode()[:1500]}\n{url}")
    if "$$SOE" not in text:
        raise SystemExit("Horizons não retornou efeméride:\n" + text[:2000])
    return [l for l in text.split("$$SOE")[1].split("$$EOE")[0].strip().splitlines()]


def table(command, out):
    n = int((JD1 - JD0) // STEP) + 1
    rows = []
    # Horizons limita o tamanho da resposta; busca em blocos.
    chunk = 1500
    for i in range(0, n, chunk):
        a = JD0 + i * STEP
        b = JD0 + (min(i + chunk, n) - 1) * STEP
        lines = horizons(COMMAND=command, EPHEM_TYPE="VECTORS", CENTER="500@10", REF_PLANE="ECLIPTIC",
                         REF_SYSTEM="ICRF", VEC_TABLE="1", OUT_UNITS="AU-D",
                         START_TIME=f"JD{a}", STOP_TIME=f"JD{b}", STEP_SIZE=f"{STEP}d")
        for l in lines:
            c = [x.strip() for x in l.split(",")]
            rows.append((float(c[0]), float(c[2]), float(c[3]), float(c[4])))
    assert len(rows) == n, (len(rows), n)
    assert abs(rows[0][0] - JD0) < 1e-6 and abs(rows[-1][0] - (JD0 + (n - 1) * STEP)) < 1e-6
    with open(out, "wb") as f:
        f.write(struct.pack("<ddQ", JD0, float(STEP), n))
        for _, x, y, z in rows:
            f.write(struct.pack("<ddd", x, y, z))
    print(out, n, "amostras")


BODIES = {"sun": "10", "moon": "301", "mercury": "199", "venus": "299", "mars": "499", "jupiter": "599",
          "saturn": "699", "uranus": "799", "neptune": "899", "pluto": "999", "chiron": "2060;"}
# Instantes em UT (dia juliano).
TIMES = [2415093.75, 2426000.125, 2433282.5, 2442413.9, 2447382.0625, 2451545.0, 2455000.3, 2460400.5, 2466154.75, 2470000.0, 2480000.25]


def reference():
    out = {}
    for name, cmd in BODIES.items():
        lines = horizons(COMMAND=cmd, EPHEM_TYPE="OBSERVER", CENTER="500@399", QUANTITIES="31",
                         TLIST=" ".join(str(t) for t in TIMES), TLIST_TYPE="JD", TIME_TYPE="UT",
                         ANG_FORMAT="DEG", CAL_FORMAT="JD", EXTRA_PREC="YES")
        vals = []
        for l in lines:
            c = [x.strip() for x in l.split(",")]
            nums = [float(x) for x in c if x and x.replace(".", "").replace("-", "").replace("+", "").replace("E", "").isdigit()]
            vals.append({"jd_ut": nums[0], "lon": nums[1], "lat": nums[2]})
        assert len(vals) == len(TIMES), (name, lines)
        out[name] = vals
        print(name, vals[5])
    p = ROOT / "crates/astro-core/tests/horizons_reference.json"
    p.write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    what = sys.argv[1] if len(sys.argv) > 1 else "all"
    if what in ("tables", "all"):
        table("9", ROOT / "crates/astro-core/data/pluto.bin")
        table("2060;", ROOT / "crates/astro-core/data/chiron.bin")
    if what in ("reference", "all"):
        reference()
