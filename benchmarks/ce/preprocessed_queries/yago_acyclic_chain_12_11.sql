select count(*) from yago1, yago5, yago22, yago0, yago17_4, yago36, yago53_6, yago53_7, yago35, yago21, yago17_10, yago17_11 where yago1.d = yago0.d and yago5.s = yago22.s and yago5.d = yago17_10.d and yago0.s = yago17_4.d and yago17_4.s = yago36.s and yago36.d = yago53_6.d and yago53_6.s = yago53_7.s and yago53_7.d = yago35.d and yago35.s = yago21.s and yago21.d = yago17_11.d and yago17_10.s = yago17_11.s;