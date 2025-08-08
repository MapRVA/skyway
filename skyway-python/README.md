# skyway for Python

```python
import skyway

skyway.convert(input="/path/to/input.osm", output="/path/to/output.json")
```

Rebuild and test:
```
maturin develop --uv && uv sync --reinstall && uv run python
```
