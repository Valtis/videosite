FROM nginx:1.29-alpine

# Use Nginx to serve static html/css/js files that is our frontend

COPY ./nginx.conf /etc/nginx/conf.d/default.conf
COPY ./app/ /usr/share/nginx/html

