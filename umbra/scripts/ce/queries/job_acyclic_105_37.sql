select count(*) from imdb100, imdb125, imdb51, imdb13 where imdb100.d = imdb125.d and imdb125.d = imdb51.s and imdb51.s = imdb13.s;