import logging
import sys, os
from aiohttp import web
from aiogram import Bot, Dispatcher, types
from aiogram.enums import ParseMode
from aiogram.filters import CommandStart
from aiogram.webhook.aiohttp_server import SimpleRequestHandler, setup_application


TOKEN = os.getenv("TOKEN")

PORT = int(os.getenv("PORT", 3001))


dp = Dispatcher()
bot = Bot(token=TOKEN)

@dp.message()
async def echo_handler(message: types.Message):
    await message.answer("three bot with webhook")

def main():
    logging.basicConfig(level=logging.INFO, stream=sys.stdout)

    app = web.Application()

    webhook_requests_handler = SimpleRequestHandler(
        dispatcher=dp,
        bot=bot,
    )

    webhook_requests_handler.register(app, path="/bot")

    setup_application(app, dp, bot=bot)


    web.run_app(app, host="0.0.0.0", port=PORT)

if __name__ == "__main__":
    main()