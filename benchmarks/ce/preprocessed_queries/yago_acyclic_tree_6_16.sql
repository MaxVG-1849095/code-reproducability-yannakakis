select count(*) from yago0, yago3, yago17, yago46, yago4, yago22 where yago0.d = yago3.d and yago3.s = yago17.d and yago17.s = yago46.s and yago46.s = yago4.d and yago46.d = yago22.s;