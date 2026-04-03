: MAIN
  S" sum: " TYPE
  3 4 + DUP PWRITE-I32 PWRITELN

  S" if: " TYPE
  1 IF
    S" true" TYPE
  ELSE
    S" false" TYPE
  THEN
  PWRITELN
;
