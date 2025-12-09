import asyncio
import logging
import sys, os
from aiogram import Bot, Dispatcher
from aiogram.client.default import DefaultBotProperties
from aiogram.enums import ParseMode
from aiogram.filters import CommandStart
from aiogram.types import Message
from aiogram.client.session.aiohttp import AiohttpSession
from aiogram.client.telegram import TelegramAPIServer
TOKEN = os.getenv("TOKEN")
TGIN_ENDPOINT = os.getenv("TGIN_ENDPOINT")
from aiogram.client.telegram import TelegramAPIServer
class TginUpateLongPullServer(TelegramAPIServer):
    tgin_updates: str
    def __init__(self, tgin_updates: str, **kwargs):
        self.tgin_updates = tgin_updates
        if "base" not in kwargs:
            kwargs["base"] = "https://api.telegram.org/bot{token}/{method}"
        if "file" not in kwargs:
            kwargs["file"] = "https://api.telegram.org/file/bot{token}/{path}"
        super().__init__(**kwargs)
    def api_url(self, token: str, method: str) -> str:
        if method == "getUpdates":
            return self.tgin_updates
        return self.base.format(token=token, method=method)
session = AiohttpSession(api=TginUpateLongPullServer(
    tgin_updates= TGIN_ENDPOINT,
))
dp = Dispatcher()

@dp.message()
async def echo_handler(message: Message) -> None:
    await message.answer("hello from first bot")
async def main() -> None:
    bot = Bot(token=TOKEN, default=DefaultBotProperties(parse_mode=ParseMode.HTML), session=session)

    await dp.start_polling(bot)

if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, stream=sys.stdout)
    asyncio.run(main())