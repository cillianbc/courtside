# openvolley oracle for the parity tests (PRD I-03).
# Pinned so oracle output only changes when we choose to bump it.
FROM rocker/r-ver:4.5.3

ARG DATAVOLLEY_REF=c23109229faae90666f5d71eafdb2e9696175d48
RUN install2.r --error remotes \
 && R -q -e "remotes::install_github('openvolley/datavolley@${DATAVOLLEY_REF}', upgrade = 'never', dependencies = NA)" \
 && R -q -e "library(datavolley); cat(as.character(packageVersion('datavolley')), '\n')"

COPY oracle.R /oracle.R
ENTRYPOINT ["Rscript", "/oracle.R"]
