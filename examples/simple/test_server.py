

from fastapi import FastAPI


app = FastAPI()


@app.post('/test')
async def test(data: str):
    print('test post', data)

@app.get('/test')
async def test(data: str):
    print('test get', data)