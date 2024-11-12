import { createServer, request } from 'node:http';
import * as fs from 'fs';

const server = createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/plain' });
  res.end('Code Recieved!\n');

  if(req.method == "GET"){
    if(req.url.includes("/callback?code")){
      console.log("Recieved code..")
      fs.writeFileSync('code.txt', req.url.substring(15), err => {
        if (err) {
          console.error(err);
        }
    });
    }
  }
});

// starts a simple http server locally on port 8888
server.listen(8888, '127.0.0.1', () => {
  console.log('Listening on 127.0.0.1:8888');
});

