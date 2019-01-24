FROM python:3.6-alpine as build-base

RUN apk add build-base

COPY requirements.txt /

RUN pip install -r /requirements.txt

COPY src/ /app

WORKDIR /app

ENTRYPOINT ["python", "./ark.py", "-s"]