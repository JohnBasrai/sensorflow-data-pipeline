FROM python:3.11-slim

WORKDIR /app

COPY data/sensor_data.json ./data/sensor_data.json
COPY main.py .

RUN pip install "fastapi" "uvicorn[standard]" "python-dotenv"

ENTRYPOINT ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8080"]
