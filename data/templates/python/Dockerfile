FROM python:3.10

RUN pip install --no-cache-dir --upgrade pip

RUN pip install --no-cache-dir pwntools cryptography beautifulsoup4 requests

WORKDIR /exploit

COPY requirements.txt /exploit/requirements.txt
RUN pip install --no-cache-dir -r /exploit/requirements.txt
COPY . /exploit

# --------------------------------------------------------------------------------------
# DONT CHANGE ANYTHING ABOVE THIS LINE UNLESS NECESSARY - WILL CAUSE LONG BUILD TIME :(




CMD [ "python3", "exploit.py" ]