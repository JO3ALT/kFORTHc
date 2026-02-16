program s18;

function Fact(n: integer): integer;
begin
  if n <= 1 then
    Fact := 1
  else
    Fact := n * Fact(n - 1)
end;

begin
  WriteLn(Fact(0));
  WriteLn(Fact(1));
  WriteLn(Fact(5))
end.
