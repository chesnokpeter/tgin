import os
import asyncio, time
from aiohttp import web
from aiogram import Bot, Dispatcher, types
from aiogram.client.session.aiohttp import AiohttpSession
from aiogram.client.telegram import TelegramAPIServer
from aiogram.webhook.aiohttp_server import SimpleRequestHandler, setup_application

TOKEN = os.getenv("BOT_TOKEN", "123:test")
API_URL = os.getenv("TELEGRAM_API_URL", "http://host.docker.internal:8090")
WEBHOOK_PATH = os.getenv("WEBHOOK_PATH", "/webhook")
PORT = int(os.getenv("PORT", 8080))

async def main():
    session = AiohttpSession(api=TelegramAPIServer.from_base(API_URL))
    bot = Bot(token=TOKEN, session=session)
    dp = Dispatcher()

    @dp.message()
    async def echo_handler(message: types.Message):
        try:
            await asyncio.sleep(0.01)
            await message.answer(message.text)
        except Exception as e:
            print(f"Error sending reply: {e}")

    app = web.Application()
    SimpleRequestHandler(dispatcher=dp, bot=bot).register(app, path=WEBHOOK_PATH)
    setup_application(app, dp, bot=bot)

    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, host="0.0.0.0", port=PORT)
    await site.start()
    
    await asyncio.Event().wait()


if __name__ == "__main__":
    asyncio.run(main())