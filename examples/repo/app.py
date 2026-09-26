import config

def total(items):
    return sum(i.price * i.qty for i in items)

def connect():
    return Database(host=config.DB_HOST, password=config.DB_PASSWORD)

def health():
    return {"status": "ok", "port": config.PORT}
