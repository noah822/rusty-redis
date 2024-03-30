import socket
from typing import List


class RESPEncoder:
    TERM = '\r\n'
    def __init__(self):
       pass
    def encode_as_bulk_str(self, items: List[str]) -> str:
      num_item = len(items)
      encoded_msg = f'*{num_item}{self.TERM}'
      for item in items:
          encoded_msg = encoded_msg + f'${len(item)}{self.TERM}{item}{self.TERM}' 
      return encoded_msg 

class RESPDecoder:
    TERM = '\r\n'
    def __init__(self):
       pass
   
    def decode(self, msg: str):
       category, msg = msg[0], msg[1:]
       if category == '+':
          return self.decode_simple_str(msg)
       elif category == '$':
          return self.decode_bulk_str(msg)
       elif category == '*':
          return self.decode_array(msg)
       else:
          return 'unacceptable format!'

    def decode_simple_str(self, msg) -> str:
        msg.split(self.TERM)[0]
   
    def decode_bulk_str(self, msg) -> str:
        msg = list(filter(lambda s: len(s) != 0, msg.split(self.TERM)))
        str_len, str_content = msg[0], msg[1]
        if int(str_len) == len(str_content):
           return str_content
        else:
           return 'unacceptable format!'

    def decode_array(self, msg) -> str:
        msg = list(filter(lambda s: len(s) != 0, msg.split(self.TERM)))
        arr_size, msg = msg[0], msg[1:]

        arr_content = msg[1::2]
        if int(arr_size) == len(arr_content):
            return '\n'.join(arr_content)
        else:
           return 'unacceptable format!'
           
        
        




class RedisShell:
    def __init__(self, bind: str):
        ip, port = bind.split(':')

        self.buf_size = 2048
        self.tcp_conn = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.tcp_conn.connect((ip, int(port)))
        self.prompt = 'redis > '
        self.encoder, self.decoder = RESPEncoder(), RESPDecoder()

    def run(self, decode_response: bool=True):
       while True:
        try:
           cmd = input(self.prompt)
           encoded_msg = self.encoder.encode_as_bulk_str(cmd.strip().split(' '))
           self.tcp_conn.send(encoded_msg.encode('utf-8'))
           response = self.tcp_conn.recv(self.buf_size).decode('utf-8')
           if decode_response:
              response = self.decoder.decode(response)
           print(response)
        except KeyboardInterrupt:
           print('Exit')
           self.tcp_conn.close()
           break


        
