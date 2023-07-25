FROM python:3.10
# not slim! we need gcc etc for the python libs

ENV JAVA_HOME=/opt/java/openjdk
COPY --from=eclipse-temurin:17-jdk $JAVA_HOME $JAVA_HOME
ENV PATH="${JAVA_HOME}/bin:${PATH}"

RUN pip install --no-cache-dir --upgrade pip

RUN pip install --no-cache-dir pwntools cryptography beautifulsoup4 requests

WORKDIR /exploit

COPY requirements.txt /exploit/requirements.txt
RUN pip install --no-cache-dir -r /exploit/requirements.txt
COPY . /exploit


# --------------------------------------------------------------------------------------
# DONT CHANGE ANYTHING ABOVE THIS LINE UNLESS NECESSARY - WILL CAUSE LONG BUILD TIME :(




CMD [ "python3", "exploit.py" ]