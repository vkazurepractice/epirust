FROM node:14.21.3-alpine
COPY package*.json ./

RUN npm install
RUN mkdir web
COPY . ./web
RUN cd web/server && npm install
RUN cd web/react-spa && npm install && npm rebuild node-sass --force
