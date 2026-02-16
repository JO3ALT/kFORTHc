program s08;
type
  point = record
    x: integer;
    y: integer;
  end;
  person = record
    age: integer;
    initial: char;
    active: boolean;
  end;
var
  p1: point;
  p2: point;
  u1: person;
  u2: person;
begin
  p1.x := 7;
  p1.y := 9;
  p2 := p1;

  u1.age := 30;
  u1.initial := 'M';
  u1.active := p2.x < p2.y;
  u2 := u1;

  WriteLn(p2.x + p2.y);
  WriteLn(u2.age);
  WriteLn(u2.initial);
  WriteLn(u2.active)
end.
