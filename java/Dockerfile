FROM maven:3.9-eclipse-temurin-21 AS build
WORKDIR /src
COPY pom.xml .
RUN mvn -q -DskipTests dependency:go-offline
COPY src ./src
RUN mvn -q -DskipTests package

FROM quay.io/wildfly/wildfly:latest
COPY --from=build /src/target/app.war /opt/jboss/wildfly/standalone/deployments/app.war
EXPOSE 8080
