

from fastapi import FastAPI


app = FastAPI()


@app.post('/test')
async def test(data: str):
    print('test post', data)
    return {'message': 'Data received successfully'}

@app.get('/test')
async def test(data: str):
    print('test get', data)
    return {'message': 'Data received successfully'}