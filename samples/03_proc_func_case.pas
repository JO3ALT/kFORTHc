program s03;
var
  g: integer;

procedure AddN(n: integer; var t: integer);
begin
  t := t + n
end;

function Classify(x: integer): integer;
begin
  case x of
    0: Classify := 100;
    1: Classify := 200
  else
    Classify := 999
  end
end;

procedure Outer(var x: integer);
  procedure Inner(var y: integer);
  begin
    y := y * 2
  end;
begin
  Inner(x)
end;

begin
  g := 5;
  AddN(7, g);
  WriteLn(g);

  Outer(g);
  WriteLn(g);

  WriteLn(Classify(0));
  WriteLn(Classify(1));
  WriteLn(Classify(7))
end.
