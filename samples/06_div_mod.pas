program s06;
var
  a: integer;
  b: integer;
begin
  a := 17;
  b := 5;

  WriteLn(a div b);
  WriteLn(a mod b);

  WriteLn(20 div 4);
  WriteLn(20 mod 4);

  WriteLn(-17 div 5);
  WriteLn(-17 mod 5);

  WriteLn(17 div -5);
  WriteLn(17 mod -5)
end.
