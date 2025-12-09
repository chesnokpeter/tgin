FROM python:3.12-slim

WORKDIR /app

RUN apt-get update && apt-get install -y curl && apt-get clean
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

ENV PATH="/root/.local/bin:$PATH"

COPY pyproject.toml uv.lock* ./

RUN uv sync

COPY . .


COPY bot1.py /app/main.py

CMD ["uv", "run", "main.py"]