select count(*) from imdb100, imdb2, imdb54 where imdb100.d = imdb2.d and imdb2.d = imdb54.s;