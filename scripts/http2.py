import asyncio
from hypercorn.config import Config
from hypercorn.asyncio import serve

from app import app

config = Config()
config = config.from_mapping({
    'bind': '127.0.0.1:4032'
})

asyncio.run(serve(app, Config()))