FROM python:3.7-alpine3.8

RUN apk add build-base

RUN python3 -m pip install --upgrade pip

COPY requirements.txt /

RUN pip3 install -r /requirements.txt

COPY src/ /app

WORKDIR /app

ENTRYPOINT ["python3", "./ark.py"]

CMD ["-s"]
