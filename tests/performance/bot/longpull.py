import asyncio
import logging
import sys, os, time
from aiogram import Bot, Dispatcher
from aiogram.client.default import DefaultBotProperties
from aiogram.enums import ParseMode
from aiogram.filters import CommandStart
from aiogram.types import Message, User
from aiogram.client.session.aiohttp import AiohttpSession
from aiogram.client.telegram import TelegramAPIServer


TGIN_ENDPOINT = os.getenv("TGIN_ENDPOINT", "http://localhost:8090/bot123:test/getUpdates")

TOKEN = os.getenv("BOT_TOKEN", "123:test")
API_URL = os.getenv("TELEGRAM_API_URL", "http://localhost:8090")



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





async def main() -> None:
    session = AiohttpSession(api=TginUpateLongPullServer(
        tgin_updates= TGIN_ENDPOINT,
        base=f"{API_URL}/bot{{token}}/{{method}}",
        file=f"{API_URL}/file/bot{{token}}/{{path}}",
    ))
    dp = Dispatcher()


    @dp.message()
    async def echo_handler(message: Message):
        try:
            await asyncio.sleep(0.01)
            await message.answer(message.text)
        except Exception as e:
            print(f"Error sending reply: {e}")




    async def myme(self) -> User:
        return User(
            id=123456789,
            is_bot=True,
            first_name="LongPullBot",
            username="long_pull_bot"
        )
    
    Bot.me = myme 


    bot = Bot(token=TOKEN, session=session)

    await dp.start_polling(bot, polling_timeout=0)

asyncio.run(main())