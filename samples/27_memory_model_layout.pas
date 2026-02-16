program s27;
type
  rec3 = record
    a: integer;
    b: integer;
    c: integer;
  end;
  arr3 = array[3] of integer;
var
  r1: rec3;
  r2: rec3;
  a1: arr3;
begin
  r1.a := 11;
  r1.b := 22;
  r1.c := 33;
  r2.a := 100;
  r2.b := 200;
  r2.c := 300;

  a1[0] := 7;
  a1[1] := 8;
  a1[2] := 9;

  WriteLn(r1.a);
  WriteLn(r1.b);
  WriteLn(r1.c);
  WriteLn(r2.a);
  WriteLn(r2.b);
  WriteLn(r2.c);
  WriteLn(a1[0]);
  WriteLn(a1[1]);
  WriteLn(a1[2]);

  r1.b := 99;
  a1[1] := 88;
  WriteLn(r1.a + r1.c);
  WriteLn(r2.a + r2.b + r2.c);
  WriteLn(a1[0] + a1[2]);
  WriteLn(r1.b);
  WriteLn(a1[1])
end.
