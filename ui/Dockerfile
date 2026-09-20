FROM node:20-slim AS builder
WORKDIR /app
COPY package.json ./
RUN npm install
COPY . .
ARG VITE_MASTER_URL=http://localhost:3000
ENV VITE_MASTER_URL=$VITE_MASTER_URL
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
