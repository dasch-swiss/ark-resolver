FROM python:3.7-alpine3.8

RUN apk add build-base

COPY requirements.txt /

RUN pip install -r /requirements.txt

COPY src/ /app

WORKDIR /app

ENTRYPOINT ["python", "./ark.py"]