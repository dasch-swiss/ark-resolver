FROM python:3.11-alpine3.20

RUN apk add --no-cache build-base

RUN python3 -m pip install --upgrade pip

COPY requirements.txt /

RUN pip3 install -r /requirements.txt

COPY src/ /app

WORKDIR /app

ENTRYPOINT ["python3", "./ark.py"]

CMD ["-s"]
