FROM python:3.13-slim
WORKDIR /app
RUN pip install aiogram aiohttp
COPY webhook.py bot.py

CMD ["python", "bot.py"]