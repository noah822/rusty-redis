from shell import RedisShell


if __name__ == '__main__':
    port = 6379
    shell = RedisShell(f'localhost:{port}')
    shell.run()

