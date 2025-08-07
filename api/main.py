import json
import os
import string
from pathlib import Path
from typing import Optional

from dotenv import load_dotenv
from fastapi import FastAPI, HTTPException, Query, Request

load_dotenv()

app = FastAPI()

DATA_FILE = Path(__file__).parent / "data/sensor_data.json"
API_KEY = os.getenv("API_KEY")
PAGE_SIZE = 100

# Base62 alphabet (alphanumeric only)
BASE62 = string.digits + string.ascii_letters


def base62_encode(num: int) -> str:
    if num == 0:
        return BASE62[0]

    result = []

    while num:
        num, rem = divmod(num, 62)
        result.append(BASE62[rem])

    return "".join(reversed(result))


def base62_decode(s: str) -> int:
    num = 0

    for char in s:
        if char not in BASE62:
            raise ValueError("Invalid base62 character")
        num = num * 62 + BASE62.index(char)

    return num


@app.get("/sensor-data")
def get_sensor_data(request: Request, cursor: Optional[str] = Query(None)):
    client_key = request.headers.get("x-api-key")
    if client_key != API_KEY:
        raise HTTPException(status_code=403, detail="Forbidden")

    with open(DATA_FILE, "r") as f:
        data = json.load(f)

    # Decode cursor to offset
    try:
        start_index = base62_decode(cursor) if cursor else 0
    except ValueError:
        raise HTTPException(status_code=400, detail="Invalid cursor")

    end_index = start_index + PAGE_SIZE
    results = data[start_index:end_index]

    next_cursor = base62_encode(end_index) if end_index < len(data) else None

    return {"results": results, "next_cursor": next_cursor}
